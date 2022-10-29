//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use futures::FutureExt;
use getset::{CopyGetters, Getters};
use log::trace;
use result_inspect::ResultInspect;
use shiplift::Container;
use shiplift::Docker;
use shiplift::ExecContainerOptions;
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::StreamExt;
use typed_builder::TypedBuilder;

use crate::config::EndpointName;
use crate::endpoint::EndpointConfiguration;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::filestore::path::ArtifactPath;
use crate::job::JobResource;
use crate::job::RunnableJob;
use crate::log::LogItem;
use crate::log::buffer_stream_to_line_stream;
use crate::package::Script;
use crate::util::docker::ContainerHash;
use crate::util::docker::ImageName;

#[derive(Getters, CopyGetters, TypedBuilder)]
pub struct Endpoint {
    #[getset(get = "pub")]
    name: EndpointName,

    #[getset(get = "pub")]
    docker: Docker,

    #[getset(get_copy = "pub")]
    num_max_jobs: usize,

    #[getset(get = "pub")]
    network_mode: Option<String>,

    #[getset(get = "pub")]
    uri: String,

    #[builder(default)]
    running_jobs: std::sync::atomic::AtomicUsize,
}

impl Debug for Endpoint {
    fn fmt(&self, f: &mut Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "Endpoint({}, max: {})", self.name, self.num_max_jobs)
    }
}

impl Endpoint {
    pub(super) async fn setup(epc: EndpointConfiguration) -> Result<Self> {
        let ep = Endpoint::setup_endpoint(epc.endpoint_name(), epc.endpoint()).with_context(|| {
            anyhow!(
                "Setting up endpoint: {} -> {}",
                epc.endpoint_name(),
                epc.endpoint().uri()
            )
        })?;

        let versions_compat =
            Endpoint::check_version_compat(epc.required_docker_versions().as_ref(), &ep);
        let api_versions_compat =
            Endpoint::check_api_version_compat(epc.required_docker_api_versions().as_ref(), &ep);
        let imgs_avail = Endpoint::check_images_available(epc.required_images().as_ref(), &ep);

        let (versions_compat, api_versions_compat, imgs_avail) = {
            let timeout = std::time::Duration::from_secs(epc.endpoint().timeout().unwrap_or(10));
            let versions_compat = tokio::time::timeout(timeout, versions_compat);
            let api_versions_compat = tokio::time::timeout(timeout, api_versions_compat);
            let imgs_avail = tokio::time::timeout(timeout, imgs_avail);
            tokio::join!(versions_compat, api_versions_compat, imgs_avail)
        };

        let _ = versions_compat.with_context(|| {
            anyhow!(
                "Checking version compatibility for {} -> {}",
                epc.endpoint_name(),
                epc.endpoint().uri()
            )
        })?;
        let _ = api_versions_compat.with_context(|| {
            anyhow!(
                "Checking API version compatibility for {} -> {}",
                epc.endpoint_name(),
                epc.endpoint().uri()
            )
        })?;
        let _ = imgs_avail.with_context(|| {
            anyhow!(
                "Checking for available images on {} -> {}",
                epc.endpoint_name(),
                epc.endpoint().uri()
            )
        })?;

        Ok(ep)
    }

    fn setup_endpoint(ep_name: &EndpointName, ep: &crate::config::Endpoint) -> Result<Endpoint> {
        match ep.endpoint_type() {
            crate::config::EndpointType::Http => shiplift::Uri::from_str(ep.uri())
                .map(shiplift::Docker::host)
                .with_context(|| anyhow!("Connecting to {}", ep.uri()))
                .map_err(Error::from)
                .map(|docker| {
                    Endpoint::builder()
                        .name(ep_name.clone())
                        .uri(ep.uri().clone())
                        .docker(docker)
                        .num_max_jobs(ep.maxjobs())
                        .network_mode(ep.network_mode().clone())
                        .build()
                }),

            crate::config::EndpointType::Socket => Ok({
                Endpoint::builder()
                    .name(ep_name.clone())
                    .uri(ep.uri().clone())
                    .num_max_jobs(ep.maxjobs())
                    .network_mode(ep.network_mode().clone())
                    .docker(shiplift::Docker::unix(ep.uri()))
                    .build()
            }),
        }
    }

    async fn check_version_compat(req: Option<&Vec<String>>, ep: &Endpoint) -> Result<()> {
        match req {
            None => Ok(()),
            Some(v) => {
                let avail = ep
                    .docker()
                    .version()
                    .await
                    .with_context(|| anyhow!("Getting version of endpoint: {}", ep.name))?;

                if !v.contains(&avail.version) {
                    Err(anyhow!(
                        "Incompatible docker version on endpoint {}: Expected: {}, Available: [{}]",
                        ep.name(),
                        avail.version,
                        v.join(", ")
                    ))
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
                let avail = ep
                    .docker()
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

    async fn check_images_available(imgs: &[ImageName], ep: &Endpoint) -> Result<()> {
        use shiplift::ImageListOptions;

        trace!("Checking availability of images: {:?}", imgs);
        let available_names = ep
            .docker()
            .images()
            .list(&ImageListOptions::builder().all().build())
            .await
            .with_context(|| anyhow!("Listing images on endpoint: {}", ep.name))?
            .into_iter()
            .flat_map(|image_rep| {
                image_rep
                    .repo_tags
                    .unwrap_or_default()
                    .into_iter()
                    .map(ImageName::from)
            })
            .collect::<Vec<ImageName>>();

        trace!("Available images = {:?}", available_names);

        imgs.iter()
            .map(|img| {
                if !available_names.contains(img) {
                    Err(anyhow!(
                        "Image '{}' missing from endpoint '{}'",
                        img.as_ref(),
                        ep.name
                    ))
                } else {
                    Ok(())
                }
            })
            .collect::<Result<Vec<_>>>()
            .map(|_| ())
    }

    pub async fn prepare_container(
        &self,
        job: RunnableJob,
        staging_store: Arc<RwLock<StagingStore>>,
        release_stores: Vec<Arc<ReleaseStore>>,
    ) -> Result<PreparedContainer<'_>> {
        PreparedContainer::new(self, job, staging_store, release_stores).await
    }

    pub fn running_jobs(&self) -> usize {
        self.running_jobs.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Super non-scientific utilization calculation for the endpoint
    pub fn utilization(&self) -> f64 {
        let max_jobs = self.num_max_jobs() as f64;
        let run_jobs = self.running_jobs() as f64;
        trace!("utilization of {}: 100.0 / {} * {}", self.name(), max_jobs, run_jobs);
        100.0 / max_jobs * run_jobs
    }

    /// Ping the endpoint (once)
    pub async fn ping(&self) -> Result<String> {
        self.docker.ping().await.map_err(Error::from)
    }

    pub async fn stats(&self) -> Result<EndpointStats> {
        self.docker
            .info()
            .await
            .map(EndpointStats::from)
            .map_err(Error::from)
    }

    pub async fn container_stats(&self) -> Result<Vec<ContainerStat>> {
        self.docker
            .containers()
            .list({
                &shiplift::builder::ContainerListOptions::builder()
                .all()
                .build()
            })
            .await
            .map_err(Error::from)
            .map(|containers| {
                containers
                    .into_iter()
                    .map(ContainerStat::from)
                    .collect()
            })
    }

    pub async fn number_of_running_containers(&self) -> Result<usize> {
        self.docker
            .containers()
            .list({
                &shiplift::builder::ContainerListOptions::builder()
                .all()
                .build()
            })
            .await
            .map_err(Error::from)
            .map(|list| {
                list.into_iter()
                    .inspect(|stat| trace!("stat = {:?}", stat))
                    .filter(|stat| stat.state == "running")
                    .count()
            })
    }

    pub async fn has_container_with_id(&self, id: &str) -> Result<bool> {
        self.container_stats()
            .await?
            .iter()
            .find(|st| st.id == id)
            .map(Ok)
            .transpose()
            .map(|o| o.is_some())
    }

    pub async fn get_container_by_id(&self, id: &str) -> Result<Option<Container<'_>>> {
        if self.has_container_with_id(id).await? {
            Ok(Some(self.docker.containers().get(id)))
        } else {
            Ok(None)
        }
    }

    pub async fn images(&self, name_filter: Option<&str>) -> Result<impl Iterator<Item = Image>> {
        let mut listopts = shiplift::builder::ImageListOptions::builder();

        if let Some(name) = name_filter {
            listopts.filter_name(name);
        } else {
            listopts.all();
        }

        self.docker
            .images()
            .list(&listopts.build())
            .await
            .map_err(Error::from)
            .map(|v| v.into_iter().map(Image::from))
    }
}

/// Helper type to store endpoint statistics
///
/// Currently, this can only be generated from a shiplift::rep::Info, but it does not hold all
/// values the shiplift::rep::Info type holds, because some of these are not relevant for us.
///
/// Later, this might hold endpoint stats from other endpoint implementations as well
pub struct EndpointStats {
    pub name: String,
    pub containers: u64,
    pub images: u64,
    pub id: String,
    pub kernel_version: String,
    pub mem_total: u64,
    pub memory_limit: bool,
    pub n_cpu: u64,
    pub operating_system: String,
    pub system_time: Option<String>,
}

impl From<shiplift::rep::Info> for EndpointStats {
    fn from(info: shiplift::rep::Info) -> Self {
        EndpointStats {
            name: info.name,
            containers: info.containers,
            images: info.images,
            id: info.id,
            kernel_version: info.kernel_version,
            mem_total: info.mem_total,
            memory_limit: info.memory_limit,
            n_cpu: info.n_cpu,
            operating_system: info.operating_system,
            system_time: info.system_time,
        }
    }
}

/// Helper type to store stats about a container
pub struct ContainerStat {
    pub created: chrono::DateTime<chrono::Utc>,
    pub id: String,
    pub image: String,
    pub image_id: String,
    pub state: String,
    pub status: String,
}

impl From<shiplift::rep::Container> for ContainerStat {
    fn from(cont: shiplift::rep::Container) -> Self {
        ContainerStat {
            created: cont.created,
            id: cont.id,
            image: cont.image,
            image_id: cont.image_id,
            state: cont.state,
            status: cont.status,
        }
    }
}

#[derive(Getters)]
pub struct Image {
    #[getset(get = "pub")]
    created: chrono::DateTime<chrono::Utc>,

    #[getset(get = "pub")]
    id: String,

    #[getset(get = "pub")]
    tags: Option<Vec<String>>,
}

impl From<shiplift::rep::Image> for Image {
    fn from(img: shiplift::rep::Image) -> Self {
        Image {
            created: img.created,
            id: img.id,
            tags: img.repo_tags,
        }
    }
}

pub struct EndpointHandle(Arc<Endpoint>);

impl EndpointHandle {
    pub fn new(ep: Arc<Endpoint>) -> Self {
        let res = ep.running_jobs.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        trace!("Endpoint {} has one job more: {}", ep.name(), res + 1);
        EndpointHandle(ep)
    }
}

impl Drop for EndpointHandle {
    fn drop(&mut self) {
        let res = self.0.running_jobs.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        trace!("Endpoint {} has one job less: {}", self.0.name(), res - 1);
    }
}

impl std::ops::Deref for EndpointHandle {
    type Target = Endpoint;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}


#[derive(Getters)]
pub struct PreparedContainer<'a> {
    endpoint: &'a Endpoint,
    script: Script,

    #[getset(get = "pub")]
    create_info: shiplift::rep::ContainerCreateInfo,
}

impl<'a> PreparedContainer<'a> {
    async fn new(
        endpoint: &'a Endpoint,
        job: RunnableJob,
        staging_store: Arc<RwLock<StagingStore>>,
        release_stores: Vec<Arc<ReleaseStore>>,
    ) -> Result<PreparedContainer<'a>> {
        let script = job.script().clone();
        let create_info = Self::build_container(endpoint, &job).await?;
        let container = endpoint.docker.containers().get(&create_info.id);

        let (cpysrc, cpypch, cpyart, cpyscr) = tokio::join!(
            Self::copy_source_to_container(&container, &job),
            Self::copy_patches_to_container(&container, &job),
            Self::copy_artifacts_to_container(&container, &job, staging_store, &release_stores),
            Self::copy_script_to_container(&container, &script)
        );

        cpysrc.with_context(|| {
            anyhow!(
                "Copying the sources to container {} on '{}'",
                create_info.id,
                endpoint.name
            )
        })?;

        cpypch.with_context(|| {
            anyhow!(
                "Copying the patches to container {} on '{}'",
                create_info.id,
                endpoint.name
            )
        })?;

        cpyart.with_context(|| {
            anyhow!(
                "Copying the artifacts to container {} on '{}'",
                create_info.id,
                endpoint.name
            )
        })?;

        cpyscr.with_context(|| {
            anyhow!(
                "Copying the script to container {} on '{}'",
                create_info.id,
                endpoint.name
            )
        })?;

        Ok({
            PreparedContainer {
                endpoint,
                script,
                create_info,
            }
        })
    }

    async fn build_container(
        endpoint: &Endpoint,
        job: &RunnableJob,
    ) -> Result<shiplift::rep::ContainerCreateInfo> {
        let envs = job
            .environment()
            .map(|(k, v)| format!("{}={}", k.as_ref(), v))
            .collect::<Vec<_>>();
        trace!("Job resources: Environment variables = {:?}", envs);

        let builder_opts = {
            let mut builder_opts = shiplift::ContainerOptions::builder(job.image().as_ref());
            let container_name = format!("butido-{package}-{version}-{id}",
                package = job.package().name().as_ref(),
                version = job.package().version().as_ref(),
                id = job.uuid()
            );
            trace!("container name = {}", container_name);
            builder_opts.name(&container_name);
            builder_opts.env(envs.iter().map(AsRef::as_ref).collect::<Vec<&str>>());
            builder_opts.cmd(vec!["/bin/bash"]); // we start the container with /bin/bash, but exec() the script in it later
            builder_opts.attach_stdin(true); // we have to attach, otherwise bash exits

            if let Some(network_mode) = endpoint.network_mode().as_ref() {
                builder_opts.network_mode(network_mode);
            }

            builder_opts.build()
        };
        trace!("Builder options = {:?}", builder_opts);

        let create_info = endpoint
            .docker
            .containers()
            .create(&builder_opts)
            .await
            .with_context(|| anyhow!("Creating container with builder options = {:?}", builder_opts))
            .with_context(|| anyhow!("Creating container on '{}'", endpoint.name))?;
        trace!("Create info = {:?}", create_info);
        Ok(create_info)
    }

    async fn copy_source_to_container<'ca>(
        container: &Container<'ca>,
        job: &RunnableJob,
    ) -> Result<()> {
        use tokio::io::AsyncReadExt;

        job.package_sources()
            .into_iter()
            .map(|entry| async {
                let source_path = entry.path();
                let destination = PathBuf::from(crate::consts::INPUTS_DIR_PATH).join({
                    source_path
                        .file_name()
                        .ok_or_else(|| anyhow!("Not a file: {}", source_path.display()))
                        .with_context(|| {
                            anyhow!(
                                "Copying package source from {} to container {}",
                                source_path.display(),
                                container.id()
                            )
                        })?
                });
                trace!("Source path    = {:?}", source_path);
                trace!("Source dest    = {:?}", destination);
                let mut buf = vec![];
                tokio::fs::OpenOptions::new()
                    .create(false)
                    .create_new(false)
                    .append(false)
                    .write(false)
                    .read(true)
                    .open(&source_path)
                    .await
                    .with_context(|| anyhow!("Getting source file: {}", source_path.display()))?
                    .read_to_end(&mut buf)
                    .await
                    .with_context(|| anyhow!("Reading file {}", source_path.display()))?;

                drop(entry);
                container.copy_file_into(destination, &buf)
                    .await
                    .inspect(|_| trace!("Successfully copied source {} to container {}", source_path.display(), container.id()))
                    .with_context(|| anyhow!("Failed to copy source {} to container {}", source_path.display(), container.id()))
                    .map_err(Error::from)
            })
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<Result<()>>()
            .await
            .inspect(|_| trace!("Successfully copied sources to container {}", container.id()))
            .with_context(|| anyhow!("Copying sources to container {}", container.id()))
            .map_err(Error::from)
    }

    async fn copy_patches_to_container<'ca>(
        container: &Container<'ca>,
        job: &RunnableJob,
    ) -> Result<()> {
        use tokio::io::AsyncReadExt;

        log::debug!("Copying patches to container: {:?}", job.package().patches());
        job.package()
            .patches()
            .iter()
            .map(|patch| async move {
                let destination = PathBuf::from(crate::consts::PATCH_DIR_PATH).join(patch);
                trace!("Copying patch {} to container at {}", patch.display(), destination.display());

                let mut buf = vec![];
                tokio::fs::OpenOptions::new()
                    .create(false)
                    .create_new(false)
                    .append(false)
                    .write(false)
                    .read(true)
                    .open(&patch)
                    .await
                    .with_context(|| anyhow!("Getting patch file: {}", patch.display()))?
                    .read_to_end(&mut buf)
                    .await
                    .with_context(|| anyhow!("Reading file {}", patch.display()))?;

                container.copy_file_into(destination, &buf)
                    .await
                    .map_err(Error::from)
                    .inspect(|_| trace!("Copying patch {} successfull", patch.display()))
                    .with_context(|| anyhow!("Copying patch {} to container {}", patch.display(), container.id()))
                    .map_err(Error::from)
            })
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<Result<()>>()
            .await
            .map_err(Error::from)
            .inspect(|_| trace!("Copied all patches"))
            .with_context(|| anyhow!("Copying patches to container {}", container.id()))
            .map_err(Error::from)
    }

    async fn copy_artifacts_to_container<'ca>(
        container: &Container<'ca>,
        job: &RunnableJob,
        staging_store: Arc<RwLock<StagingStore>>,
        release_stores: &[Arc<ReleaseStore>],
    ) -> Result<()> {
        let stream = job.resources()
            .iter()
            .filter_map(JobResource::artifact)
            .cloned()
            .map(|art| async {
                let artifact_file_name = art
                    .file_name()
                    .ok_or_else(|| anyhow!("BUG: artifact {} is not a file", art.display()))
                    .with_context(|| {
                        anyhow!(
                            "Collecting artifacts for copying to container {}",
                            container.id()
                        )
                    })?;
                let destination = PathBuf::from(crate::consts::INPUTS_DIR_PATH).join(artifact_file_name);
                trace!(
                    "Copying {} to container: {}:{}",
                    art.display(),
                    container.id(),
                    destination.display()
                );
                let staging_read = staging_store.read().await;
                let buf = match staging_read.root_path().join(&art)?  {
                    Some(fp) => fp,
                    None     => {
                        // TODO: Optimize.
                        // I know this is not nice, but it works for now.
                        let mut found = None;
                        for release_store in release_stores.iter() {
                            let p = release_store.root_path().join(&art);
                            match p {
                                Ok(Some(path)) => {
                                    found = Some(path);
                                    break;
                                },
                                Err(e) => {
                                    trace!("Failed to join '{:?}' + '{:?}'", release_store.root_path(), art.display());
                                    return Err(e)
                                },
                                Ok(None) =>  continue,
                            }
                        }
                        found.ok_or_else(|| anyhow!("Not found in staging or release store: {:?}", art))?
                    },
                }
                .read()
                .await
                .with_context(|| {
                    anyhow!(
                        "Reading artifact {}, so it can be copied to container",
                        art.display()
                    )
                })?;
                trace!("Successfully read {} into buffer", art.display());

                let r = container
                    .copy_file_into(&destination, &buf)
                    .await
                    .inspect(|_| trace!("Successfully copied {} to container", art.display()))
                    .with_context(|| {
                        anyhow!(
                            "Copying artifact {} to container {} at {}",
                            art.display(),
                            container.id(),
                            destination.display()
                        )
                    })
                    .map_err(Error::from);
                drop(art); // ensure `art` is moved into closure
                r
            });

        let stream = {
            use futures::stream::StreamExt;
            futures::stream::iter(stream).buffer_unordered(100)
        };

        stream
            .collect::<Result<Vec<_>>>()
            .await
            .inspect(|_| trace!("Successfully copied all artifacts to the container {}", container.id()))
            .with_context(|| anyhow!("Copying artifacts to container {}", container.id()))
            .map_err(Error::from)
            .map(|_| ())
    }

    async fn copy_script_to_container<'ca>(
        container: &Container<'ca>,
        script: &Script,
    ) -> Result<()> {
        let script_path = PathBuf::from(crate::consts::SCRIPT_PATH);
        container
            .copy_file_into(script_path, script.as_ref().as_bytes())
            .await
            .inspect(|_| trace!("Successfully copied script to container {}", container.id()))
            .with_context(|| anyhow!("Copying the script into container {}", container.id()))
            .map_err(Error::from)
    }

    pub async fn start(self) -> Result<StartedContainer<'a>> {
        self.endpoint
            .docker
            .containers()
            .get(&self.create_info.id)
            .start()
            .inspect(|r| trace!("Starting container {} -> {:?}", self.create_info.id, r))
            .map(|r| {
                r.with_context(|| {
                    anyhow!(
                        "Starting the container {} on '{}'",
                        self.create_info.id,
                        self.endpoint.name
                    )
                })
            })
            .await?;

        Ok({
            StartedContainer {
                endpoint: self.endpoint,
                script: self.script,
                create_info: self.create_info,
            }
        })
    }
}

pub struct StartedContainer<'a> {
    endpoint: &'a Endpoint,
    script: Script,
    create_info: shiplift::rep::ContainerCreateInfo,
}

impl<'a> StartedContainer<'a> {
    pub async fn execute_script(
        self,
        logsink: UnboundedSender<LogItem>,
    ) -> Result<ExecutedContainer<'a>> {
        let exec_opts = ExecContainerOptions::builder()
            .cmd(vec!["/bin/bash", "/script"])
            .attach_stderr(true)
            .attach_stdout(true)
            .build();
        trace!("Exec options = {:?}", exec_opts);

        trace!("Moving logs to log sink for container {}", self.create_info.id);
        let stream = self.endpoint
            .docker
            .containers()
            .get(&self.create_info.id)
            .exec(&exec_opts);

        let exited_successfully: Option<(bool, Option<String>)> =
            buffer_stream_to_line_stream(stream)
                .map(|line| {
                    trace!(
                        "['{}':{}] Found log line: {:?}",
                        self.endpoint.name,
                        self.create_info.id,
                        line
                    );
                    line.with_context(|| {
                        anyhow!(
                            "Getting log from {}:{}",
                            self.endpoint.name,
                            self.create_info.id
                        )
                    })
                    .and_then(|l| {
                        crate::log::parser()
                            .parse(l.as_bytes())
                            .with_context(|| {
                                anyhow!(
                                    "Parsing log from {}:{}: {:?}",
                                    self.endpoint.name,
                                    self.create_info.id,
                                    l
                                )
                            })
                    })
                    .and_then(|item| {
                        let exited_successfully = match item {
                            LogItem::State(Ok(_)) => Some((true, None)),
                            LogItem::State(Err(ref msg)) => Some((false, Some(msg.clone()))),
                            _ => None, // Nothing
                        };

                        trace!("Log item: {}", item.display()?);
                        logsink
                            .send(item)
                            .with_context(|| anyhow!("Sending log to log sink"))
                            .map(|_| exited_successfully)
                    })
                    .map_err(Error::from)
                })
                .collect::<Result<Vec<_>>>()
                .map(|r| {
                    r.with_context(|| {
                        anyhow!(
                            "Fetching log from container {} on {}",
                            self.create_info.id,
                            self.endpoint.name
                        )
                    })
                })
                .await
                .with_context(|| {
                    anyhow!(
                        "Copying script to container, running container and getting logs: {}",
                        self.create_info.id
                    )
                })?
                .into_iter()
                .fold(None, |accu, elem| match (accu, elem) {
                    (None, b) => b,
                    (Some((false, msg)), _) => Some((false, msg)),
                    (_, Some((false, msg))) => Some((false, msg)),
                    (a, None) => a,
                    (Some((true, _)), Some((true, _))) => Some((true, None)),
                });

        Ok({
            ExecutedContainer {
                endpoint: self.endpoint,
                create_info: self.create_info,
                script: self.script,
                exit_info: exited_successfully,
            }
        })
    }
}

pub struct ExecutedContainer<'a> {
    endpoint: &'a Endpoint,
    create_info: shiplift::rep::ContainerCreateInfo,
    script: Script,
    exit_info: Option<(bool, Option<String>)>,
}

impl<'a> ExecutedContainer<'a> {
    pub fn container_hash(&self) -> ContainerHash {
        ContainerHash::from(self.create_info.id.clone())
    }

    pub fn script(&self) -> &Script {
        &self.script
    }

    pub async fn finalize(self, staging_store: Arc<RwLock<StagingStore>>) -> Result<FinalizedContainer> {
        let (exit_info, artifacts) = match self.exit_info {
            Some((false, msg)) => {
                let err = anyhow!("Error during container run: '{msg}'", msg = msg.as_deref().unwrap_or(""));

                // error because the container errored
                (Err(err), vec![])
            }

            Some((true, _)) | None => {
                let container = self.endpoint.docker.containers().get(&self.create_info.id);

                trace!("Fetching {} from container {}", crate::consts::OUTPUTS_DIR_PATH, self.create_info.id);
                let tar_stream = container
                    .copy_from(&PathBuf::from(crate::consts::OUTPUTS_DIR_PATH))
                    .map(|item| {
                        item.with_context(|| {
                            anyhow!(
                                "Copying item from container {} to host",
                                self.create_info.id
                            )
                        })
                        .map_err(Error::from)
                    });

                let mut writelock = staging_store.write().await;
                let artifacts = writelock
                    .write_files_from_tar_stream(tar_stream)
                    .await
                    .with_context(|| anyhow!("Copying the TAR stream to the staging store"))?;
                container
                    .stop(Some(std::time::Duration::new(1, 0)))
                    .await
                    .with_context(|| anyhow!("Stopping container {}", self.create_info.id))?;
                (Ok(()), artifacts)
            }
        };

        Ok({
            FinalizedContainer {
                artifacts,
                exit_info,
            }
        })
    }
}

#[derive(Debug)]
pub struct FinalizedContainer {
    artifacts: Vec<ArtifactPath>,
    exit_info: Result<()>,
}

impl FinalizedContainer {
    pub fn unpack(self) -> (Vec<ArtifactPath>, Result<()>) {
        (self.artifacts, self.exit_info)
    }
}
