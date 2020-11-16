use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::result::Result as RResult;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use getset::{Getters, CopyGetters};
use shiplift::Docker;
use shiplift::ExecContainerOptions;
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedSender;
use typed_builder::TypedBuilder;

use crate::endpoint::EndpointConfiguration;
use crate::filestore::StagingStore;
use crate::job::JobResource;
use crate::job::RunnableJob;
use crate::log::LogItem;
use crate::package::Script;
use crate::util::docker::ContainerHash;
use crate::util::docker::ImageName;
use crate::endpoint::ContainerError;

#[derive(Getters, CopyGetters, TypedBuilder)]
pub struct Endpoint {
    #[getset(get = "pub")]
    name: String,

    #[getset(get = "pub")]
    docker: Docker,

    #[getset(get_copy = "pub")]
    speed: usize,

    #[getset(get_copy = "pub")]
    num_max_jobs: usize,

    #[getset(get = "pub")]
    uri: String,
}

impl Debug for Endpoint {
    fn fmt(&self, f: &mut Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "Endpoint({}, max: {})", self.name, self.num_max_jobs)
    }
}

impl Endpoint {

    pub(in super) async fn setup(epc: EndpointConfiguration) -> Result<Self> {
        let ep = Endpoint::setup_endpoint(epc.endpoint())
            .with_context(|| anyhow!("Setting up endpoint: {} -> {}", epc.endpoint().name(), epc.endpoint().uri()))?;

        let versions_compat     = Endpoint::check_version_compat(epc.required_docker_versions().as_ref(), &ep);
        let api_versions_compat = Endpoint::check_api_version_compat(epc.required_docker_api_versions().as_ref(), &ep);
        let imgs_avail          = Endpoint::check_images_available(epc.required_images().as_ref(), &ep);

        let (versions_compat, api_versions_compat, imgs_avail) =
            tokio::join!(versions_compat, api_versions_compat, imgs_avail);

        let _ = versions_compat
            .with_context(|| anyhow!("Checking version compatibility for {} -> {}", epc.endpoint().name(), epc.endpoint().uri()))?;
        let _ = api_versions_compat
            .with_context(|| anyhow!("Checking API version compatibility for {} -> {}", epc.endpoint().name(), epc.endpoint().uri()))?;
        let _ = imgs_avail
            .with_context(|| anyhow!("Checking for available images on {} -> {}", epc.endpoint().name(), epc.endpoint().uri()))?;

        Ok(ep)
    }

    fn setup_endpoint(ep: &crate::config::Endpoint) -> Result<Endpoint> {
        match ep.endpoint_type() {
            crate::config::EndpointType::Http => {
                shiplift::Uri::from_str(ep.uri())
                    .map(|uri| shiplift::Docker::host(uri))
                    .with_context(|| anyhow!("Connecting to {}", ep.uri()))
                    .map_err(Error::from)
                    .map(|docker| {
                        Endpoint::builder()
                            .name(ep.name().clone())
                            .uri(ep.uri().clone())
                            .docker(docker)
                            .speed(ep.speed())
                            .num_max_jobs(ep.maxjobs())
                            .build()
                    })
            }

            crate::config::EndpointType::Socket => {
                Ok({
                    Endpoint::builder()
                        .name(ep.name().clone())
                        .uri(ep.uri().clone())
                        .speed(ep.speed())
                        .num_max_jobs(ep.maxjobs())
                        .docker(shiplift::Docker::unix(ep.uri()))
                        .build()
                })
            }
        }
    }

    async fn check_version_compat(req: Option<&Vec<String>>, ep: &Endpoint) -> Result<()> {
        match req {
            None => Ok(()),
            Some(v) => {
                let avail = ep.docker()
                    .version()
                    .await
                    .with_context(|| anyhow!("Getting version of endpoint: {}", ep.name))?;

                if !v.contains(&avail.version) {
                    Err(anyhow!("Incompatible docker version on endpoint {}: Expected: {}, Available: [{}]",
                            ep.name(), avail.version, v.join(", ")))
                } else {
                    Ok(())
                }
            }
        }
    }

    async fn check_api_version_compat(req: Option<&Vec<String>>, ep: &Endpoint) -> Result<()> {
        match req {
            None => Ok(()),
            Some(v) => {
                let avail = ep.docker()
                    .version()
                    .await
                    .with_context(|| anyhow!("Getting API version of endpoint: {}", ep.name))?;

                if !v.contains(&avail.api_version) {
                    Err(anyhow!("Incompatible docker API version on endpoint {}: Exepected: {}, Available: [{}]",
                            ep.name(), avail.api_version, v.join(", ")))
                } else {
                    Ok(())
                }
            }
        }
    }

    async fn check_images_available(imgs: &Vec<ImageName>, ep: &Endpoint) -> Result<()> {
        use shiplift::ImageListOptions;

        trace!("Checking availability of images: {:?}", imgs);
        let available_names = ep
            .docker()
            .images()
            .list(&ImageListOptions::builder().all().build())
            .await
            .with_context(|| anyhow!("Listing images on endpoint: {}", ep.name))?
            .into_iter()
            .map(|image_rep| {
                image_rep.repo_tags
                    .unwrap_or_default()
                    .into_iter()
                    .map(ImageName::from)
            })
            .flatten()
            .collect::<Vec<ImageName>>();

        trace!("Available images = {:?}", available_names);

        imgs.iter()
            .map(|img| {
                if !available_names.contains(img) {
                    Err(anyhow!("Image '{}' missing from endpoint '{}'", img.as_ref(), ep.name))
                } else {
                    Ok(())
                }
            })
            .collect::<Result<Vec<_>>>()
            .map(|_| ())
    }

    pub async fn run_job(&self, job: RunnableJob, logsink: UnboundedSender<LogItem>, staging: Arc<RwLock<StagingStore>>) -> RResult<(Vec<PathBuf>, ContainerHash, Script), ContainerError> {
        use crate::log::buffer_stream_to_line_stream;
        use tokio::stream::StreamExt;
        use futures::FutureExt;

        let (container_id, _warnings) = {
            let envs = job.environment()
                .into_iter()
                .chain(job.package_environment().into_iter())
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>();
            trace!("Job resources: Environment variables = {:?}", envs);

            let builder_opts = shiplift::ContainerOptions::builder(job.image().as_ref())
                .env(envs.iter().map(AsRef::as_ref).collect())
                .cmd(vec!["/bin/bash"]) // we start the container with /bin/bash, but exec() the script in it later
                .attach_stdin(true) // we have to attach, otherwise bash exits
                .build();
            trace!("Builder options = {:?}", builder_opts);

            let create_info = self.docker
                .containers()
                .create(&builder_opts)
                .await
                .with_context(|| anyhow!("Creating container on '{}'", self.name))?;
            trace!("Create info = {:?}", create_info);

            if let Some(warnings) = create_info.warnings.as_ref() {
                for warning in warnings {
                    warn!("{}", warning);
                }
            }

            (create_info.id, create_info.warnings)
        };

        let script_path = PathBuf::from("/script");
        let exec_opts = ExecContainerOptions::builder()
            .cmd(vec!["/bin/bash", "/script"])
            .attach_stderr(true)
            .attach_stdout(true)
            .build();
        trace!("Exec options = {:?}", exec_opts);

        let container = self.docker.containers().get(&container_id);
        trace!("Container id = {:?}", container_id);
        { // copy source to container
            use tokio::io::AsyncReadExt;

            let pkgsource   = job.package_source();
            let source_path = pkgsource.path();
            let destination = PathBuf::from("/inputs").join({
                source_path.file_name()
                    .ok_or_else(|| anyhow!("Not a file: {}", source_path.display()))
                    .with_context(|| anyhow!("Copying package source from {} to container {}", source_path.display(), self.name))?
            });
            trace!("Package source = {:?}", pkgsource);
            trace!("Source path    = {:?}", source_path);
            trace!("Source dest    = {:?}", destination);
            let mut buf = vec![];
            tokio::fs::OpenOptions::new()
                .create(false)
                .create_new(false)
                .append(false)
                .write(false)
                .read(true)
                .open(source_path)
                .await
                .with_context(|| anyhow!("Getting source file: {}", source_path.display()))?
                .read_to_end(&mut buf)
                .await
                .with_context(|| anyhow!("Reading file {}", source_path.display()))?;

            let _ = container.copy_file_into(destination, &buf)
                .await
                .with_context(|| anyhow!("Copying {} to container {}", source_path.display(), container_id))?;
        }
        { // Copy all Path artifacts to the container
            job.resources()
                .into_iter()
                .filter_map(JobResource::artifact)
                .cloned()
                .map(|art| async {
                    let artifact_file_name = art.path().file_name()
                        .ok_or_else(|| anyhow!("BUG: artifact {} is not a file", art.path().display()))
                        .with_context(|| anyhow!("Collecting artifacts for copying to container {}", container_id))?;
                    let destination = PathBuf::from("/inputs/").join(artifact_file_name);
                    trace!("Copying {} to container: {}:{}", art.path().display(), container_id, destination.display());
                    let buf = tokio::fs::read(art.path())
                        .await
                        .map(Vec::from)
                        .with_context(|| anyhow!("Reading artifact {}, so it can be copied to container", art.path().display()))
                        .map_err(Error::from)?;

                    let r = container.copy_file_into(&destination, &buf)
                        .await
                        .with_context(|| anyhow!("Copying artifact {} to container {} at {}", art.path().display(), container_id, destination.display()))
                        .map_err(Error::from);
                    drop(art); // ensure `art` is moved into closure
                    r
                })
                .collect::<futures::stream::FuturesUnordered<_>>()
                .collect::<Result<Vec<_>>>()
                .await
                .with_context(|| anyhow!("Copying artifacts to container {}", container_id))?;
            }

        let exited_successfully: Option<bool> = container
            .copy_file_into(script_path, job.script().as_ref().as_bytes())
            .inspect(|r| { trace!("Copying script to container {} -> {:?}", container_id, r); })
            .map(|r| r.with_context(|| anyhow!("Copying the script into the container {} on '{}'", container_id, self.name)))
            .then(|_| container.start())
            .inspect(|r| { trace!("Starting container {} -> {:?}", container_id, r); })
            .map(|r| r.with_context(|| anyhow!("Starting the container {} on '{}'", container_id, self.name)))
            .then(|_| {
                use futures::FutureExt;
                trace!("Moving logs to log sink for container {}", container_id);
                buffer_stream_to_line_stream(container.exec(&exec_opts))
                    .map(|line| {
                        trace!("['{}':{}] Found log line: {:?}", self.name, container_id, line);
                        line.with_context(|| anyhow!("Getting log from {}:{}", self.name, container_id))
                            .map_err(Error::from)
                            .and_then(|l| {
                                crate::log::parser()
                                    .parse(l.as_bytes())
                                    .with_context(|| anyhow!("Parsing log from {}:{}: {:?}", self.name, container_id, l))
                                    .map_err(Error::from)
                                    .and_then(|item| {

                                        let mut exited_successfully = None;
                                        {
                                            match item {
                                                LogItem::State(Ok(_))    => exited_successfully = Some(true),
                                                LogItem::State(Err(_))   => exited_successfully = Some(false),
                                                _ => {
                                                    // Nothing
                                                }
                                            }
                                        }

                                        trace!("Log item: {}", item.display()?);
                                        logsink.send(item)
                                            .with_context(|| anyhow!("Sending log to log sink"))
                                            .map_err(Error::from)
                                            .map(|_| exited_successfully)
                                    })
                            })
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .map(|r| r.with_context(|| anyhow!("Fetching log from container {} on {}", container_id, self.name)))
            .await
            .with_context(|| anyhow!("Copying script to container, running container and getting logs: {}", container_id))?
            .into_iter()
            .fold(None, |accu, elem| match (accu, elem) {
                (None        , b)           => b,
                (Some(false) , _)           => Some(false),
                (_           , Some(false)) => Some(false),
                (a           , None)        => a,
                (Some(true)  , Some(true))  => Some(true),
            });

        trace!("Fetching /outputs from container {}", container_id);
        let tar_stream = container
            .copy_from(&PathBuf::from("/outputs/"))
            .map(|item| {
                item.with_context(|| anyhow!("Copying item from container {} to host", container_id))
                    .map_err(Error::from)
            });

        let r = {
            let mut writelock = staging.write().await;

            writelock
                .write_files_from_tar_stream(tar_stream)
                .await
                .with_context(|| anyhow!("Copying the TAR stream to the staging store"))?
        };

        let script: Script = job.script().clone();
        match exited_successfully {
            Some(false)       => Err(ContainerError::container_error(ContainerHash::from(container_id), self.uri().clone())),
            Some(true) | None => {
                container.stop(Some(std::time::Duration::new(1, 0)))
                    .await
                    .with_context(|| anyhow!("Stopping container {}", container_id))?;

                Ok((r, ContainerHash::from(container_id), script))
            },
        }
    }

    pub async fn number_of_running_containers(&self) -> Result<usize> {
        self.docker
            .containers()
            .list(&Default::default())
            .await
            .with_context(|| anyhow!("Getting number of running containers on {}", self.name))
            .map_err(Error::from)
            .map(|list| list.len())
    }

}

