use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::RwLock;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use getset::{Getters, CopyGetters};
use shiplift::Docker;
use shiplift::ExecContainerOptions;
use tokio::sync::mpsc::UnboundedSender;
use typed_builder::TypedBuilder;

use crate::endpoint::EndpointConfiguration;
use crate::filestore::StagingStore;
use crate::job::JobResource;
use crate::job::RunnableJob;
use crate::log::LogItem;
use crate::util::docker::ImageName;

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
}

impl Debug for Endpoint {
    fn fmt(&self, f: &mut Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "Endpoint({}, max: {})", self.name, self.num_max_jobs)
    }
}

impl Endpoint {

    pub(in super) async fn setup(epc: EndpointConfiguration) -> Result<Self> {
        let ep = Endpoint::setup_endpoint(epc.endpoint())?;

        let versions_compat     = Endpoint::check_version_compat(epc.required_docker_versions().as_ref(), &ep);
        let api_versions_compat = Endpoint::check_api_version_compat(epc.required_docker_api_versions().as_ref(), &ep);
        let imgs_avail          = Endpoint::check_images_available(epc.required_images().as_ref(), &ep);

        let (versions_compat, api_versions_compat, imgs_avail) =
            tokio::join!(versions_compat, api_versions_compat, imgs_avail);

        let _ = versions_compat?;
        let _ = api_versions_compat?;
        let _ = imgs_avail?;

        Ok(ep)
    }

    fn setup_endpoint(ep: &crate::config::Endpoint) -> Result<Endpoint> {
        match ep.endpoint_type() {
            crate::config::EndpointType::Http => {
                shiplift::Uri::from_str(ep.uri())
                    .map(|uri| shiplift::Docker::host(uri))
                    .map_err(Error::from)
                    .map(|docker| {
                        Endpoint::builder()
                            .name(ep.name().clone())
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
                    Err(anyhow!("Incompatible docker version on endpoint {}: {}",
                            ep.name(), avail.version))
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
                    Err(anyhow!("Incompatible docker API version on endpoint {}: {}",
                            ep.name(), avail.api_version))
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

    pub async fn run_job(&self, job: RunnableJob, logsink: UnboundedSender<LogItem>, staging: Arc<RwLock<StagingStore>>) -> Result<Vec<PathBuf>>  {
        use crate::log::buffer_stream_to_line_stream;
        use tokio::stream::StreamExt;
        use futures::FutureExt;

        let (container_id, _warnings) = {
            let envs = job.resources()
                .iter()
                .filter_map(JobResource::env)
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>();

            let builder_opts = shiplift::ContainerOptions::builder(job.image().as_ref())
                    .env(envs.iter().map(AsRef::as_ref).collect())
                    .cmd(vec!["/bin/bash"]) // we start the container with /bin/bash, but exec() the script in it later
                    .attach_stdin(true) // we have to attach, otherwise bash exits
                    .build();

            let create_info = self.docker
                .containers()
                .create(&builder_opts)
                .await
                .with_context(|| anyhow!("Creating container on '{}'", self.name))?;

            if let Some(warnings) = create_info.warnings.as_ref() {
                for warning in warnings {
                    warn!("{}", warning);
                }
            }

            (create_info.id, create_info.warnings)
        };

        let script      = job.script().as_ref().as_bytes();
        let script_path = PathBuf::from("/script");
        let exec_opts = ExecContainerOptions::builder()
            .cmd(vec!["/bin/bash", "/script"])
            .attach_stderr(true)
            .attach_stdout(true)
            .build();

        let container = self.docker.containers().get(&container_id);
        { // Copy all Path artifacts to the container
            job.resources()
                .into_iter()
                .filter_map(JobResource::artifact)
                .cloned()
                .map(|art| async {
                    let artifact_file_name = art.path().file_name()
                        .ok_or_else(|| anyhow!("BUG: artifact {} is not a file", art.path().display()))?;
                    let destination = PathBuf::from("/inputs/").join(artifact_file_name);
                    trace!("Copying {} to container: {}:{}", art.path().display(), container_id, destination.display());
                    let buf = tokio::fs::read(art.path())
                        .await
                        .map(Vec::from)
                        .map_err(Error::from)?;

                    drop(art); // ensure `art` is moved into closure
                    container.copy_file_into(destination, &buf)
                        .await
                        .map_err(Error::from)
                })
                .collect::<futures::stream::FuturesUnordered<_>>()
                .collect::<Result<Vec<_>>>()
                .await?;
        }

        container
            .copy_file_into(script_path, script)
            .inspect(|r| { trace!("Copying script to container {} -> {:?}", container_id, r); })
            .map(|r| r.with_context(|| anyhow!("Copying the script into the container {} on '{}'", container_id, self.name)))
            .then(|_| container.start())
            .inspect(|r| { trace!("Starting container {} -> {:?}", container_id, r); })
            .map(|r| r.with_context(|| anyhow!("Starting the container {} on '{}'", container_id, self.name)))
            .then(|_| {
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
                                        logsink.send(item)
                                            .with_context(|| anyhow!("Sending log to log sink"))
                                            .map_err(Error::from)
                                    })
                            })
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .inspect(|r| { trace!("Fetching log from container {} -> {:?}", container_id, r); })
            .map(|r| r.with_context(|| anyhow!("Fetching log from container {} on {}", container_id, self.name)))
            .await?;

        trace!("Fetching /outputs from container {}", container_id);
        let tar_stream = container
            .copy_from(&PathBuf::from("/outputs/"))
            .map(|item| item.map_err(Error::from));

        let r = staging
            .write()
            .map_err(|_| anyhow!("Lock poisoned"))?
            .write_files_from_tar_stream(tar_stream)
            .await
            .with_context(|| anyhow!("Copying the TAR stream to the staging store"))?;

        container.stop(Some(std::time::Duration::new(1, 0))).await?;

        trace!("Returning job {} result = {:?}", job.uuid(), r);
        Ok(r)
    }

    pub async fn number_of_running_containers(&self) -> Result<usize> {
        self.docker
            .containers()
            .list(&Default::default())
            .await
            .map_err(Error::from)
            .map(|list| list.len())
    }

}
