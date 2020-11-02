use std::sync::Arc;
use std::sync::RwLock;
use std::str::FromStr;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Result;
use anyhow::Error;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::UnboundedReceiver;
use shiplift::ExecContainerOptions;
use shiplift::ContainerOptions;

use crate::util::docker::ImageName;
use crate::endpoint::configured::ConfiguredEndpoint;
use crate::endpoint::managerconf::EndpointManagerConfiguration;
use crate::job::RunnableJob;
use crate::job::JobResource;
use crate::log::LogItem;
use crate::filestore::StagingStore;

/// The EndpointManager manages a _single_ endpoint
#[derive(Clone, Debug)]
pub struct EndpointManager {
    inner: Arc<RwLock<Inner>>,
}


#[derive(Debug)]
struct Inner {
    endpoint: ConfiguredEndpoint,
}

impl EndpointManager {
    pub(in super) async fn setup(epc: EndpointManagerConfiguration) -> Result<Self> {
        let ep = EndpointManager::setup_endpoint(epc.endpoint())?;

        let versions_compat     = EndpointManager::check_version_compat(epc.required_docker_versions().as_ref(), &ep);
        let api_versions_compat = EndpointManager::check_api_version_compat(epc.required_docker_api_versions().as_ref(), &ep);
        let imgs_avail          = EndpointManager::check_images_available(epc.required_images().as_ref(), &ep);

        tokio::try_join!(versions_compat, api_versions_compat, imgs_avail)?;

        Ok(EndpointManager {
            inner: Arc::new(RwLock::new(Inner {
                endpoint: ep
            }))
        })
    }

    fn setup_endpoint(ep: &crate::config::Endpoint) -> Result<ConfiguredEndpoint> {
        match ep.endpoint_type() {
            crate::config::EndpointType::Http => {
                shiplift::Uri::from_str(ep.uri())
                    .map(|uri| shiplift::Docker::host(uri))
                    .map_err(Error::from)
                    .map(|docker| {
                        ConfiguredEndpoint::builder()
                            .name(ep.name().clone())
                            .docker(docker)
                            .speed(ep.speed())
                            .num_current_jobs(0)
                            .num_max_jobs(ep.maxjobs())
                            .build()
                    })
            }

            crate::config::EndpointType::Socket => {
                Ok({
                    ConfiguredEndpoint::builder()
                        .name(ep.name().clone())
                        .speed(ep.speed())
                        .num_current_jobs(0)
                        .num_max_jobs(ep.maxjobs())
                        .docker(shiplift::Docker::unix(ep.uri()))
                        .build()
                })
            }
        }
    }

    async fn check_version_compat(req: Option<&Vec<String>>, ep: &ConfiguredEndpoint) -> Result<()> {
        match req {
            None => Ok(()),
            Some(v) => {
                let avail = ep.docker().version().await?;

                if !v.contains(&avail.version) {
                    Err(anyhow!("Incompatible docker version on endpoint {}: {}",
                            ep.name(), avail.version))
                } else {
                    Ok(())
                }
            }
        }
    }

    async fn check_api_version_compat(req: Option<&Vec<String>>, ep: &ConfiguredEndpoint) -> Result<()> {
        match req {
            None => Ok(()),
            Some(v) => {
                let avail = ep.docker().version().await?;

                if !v.contains(&avail.api_version) {
                    Err(anyhow!("Incompatible docker API version on endpoint {}: {}",
                            ep.name(), avail.api_version))
                } else {
                    Ok(())
                }
            }
        }
    }

    async fn check_images_available(imgs: &Vec<ImageName>, ep: &ConfiguredEndpoint) -> Result<()> {
        use shiplift::ImageListOptions;

        ep.docker()
            .images()
            .list(&ImageListOptions::builder().all().build())
            .await?
            .into_iter()
            .map(|image_rep| {
                if let Some(tags) = image_rep.repo_tags {
                    tags.into_iter().map(|name| {
                        let tag = ImageName::from(name.clone());

                        if imgs.iter().any(|img| *img == tag) && !imgs.is_empty() {
                            return Err(anyhow!("Image {} missing in endpoint {}", name, ep.name()))
                        }

                        Ok(())
                    })
                    .collect::<Result<()>>()?;
                    }
                // If no tags, ignore

                Ok(())
            })
        .collect::<Result<()>>()
    }

    /// Get the name of the endpoint
    pub fn endpoint_name(&self) -> Result<String> {
        self.inner.read().map(|rw| rw.endpoint.name().clone()).map_err(|_| lockpoisoned())
    }

    pub fn get_num_current_jobs(&self) -> Result<usize> {
        self.inner
            .read()
            .map(|lock| lock.endpoint.num_current_jobs())
            .map_err(|_| anyhow!("Lock poisoned"))
    }

    pub fn get_num_max_jobs(&self) -> Result<usize> {
        self.inner
            .read()
            .map(|lock| lock.endpoint.num_max_jobs())
            .map_err(|_| anyhow!("Lock poisoned"))
    }

    pub async fn run_job(self, job: RunnableJob, logsink: UnboundedSender<LogItem>, staging: Arc<RwLock<StagingStore>>) -> Result<Vec<PathBuf>>  {
        use crate::log::buffer_stream_to_line_stream;
        use tokio::stream::StreamExt;

        let lock = self.inner
            // Taking write lock, because we alter interior here, which shouldn't happen in
            // parallel eventhough technically possible.
            .write()
            .map_err(|_| anyhow!("Lock poisoned"))?;

        let (container_id, _warnings) = {
            let envs: Vec<String> = job.resources()
                .iter()
                .filter_map(|r| match r {
                    JobResource::Environment(k, v) => Some(format!("{}={}", k, v)),
                    JobResource::Path(_)           => None,
                })
                .collect();

            let builder_opts = shiplift::ContainerOptions::builder(job.image().as_ref())
                    .env(envs.iter().map(AsRef::as_ref).collect())
                    .build();

            let create_info = lock.endpoint
                .docker()
                .containers()
                .create(&builder_opts)
                .await?;

            if create_info.warnings.is_some() {
                // TODO: Handle warnings
            } 

            (create_info.id, create_info.warnings)
        };

        let script      = job.script().as_ref().as_bytes();
        let script_path = PathBuf::from("/script");
        let exec_opts   = ExecContainerOptions::builder()
            .cmd(vec!["/script"])
            .build();

        let container = lock.endpoint.docker().containers().get(&container_id);
        container.copy_file_into(script_path, script).await?;

        let stream = container.exec(&exec_opts);
        let _ = buffer_stream_to_line_stream(stream)
            .map(|line| {
                line.map_err(Error::from)
                    .and_then(|l| {
                        crate::log::parser()
                            .parse(l.as_bytes())
                            .map_err(Error::from)
                            .and_then(|item| logsink.send(item).map_err(Error::from))
                    })
            })
            .collect::<Result<Vec<_>>>()
            .await?;

        let tar_stream = container.copy_from(&PathBuf::from("/outputs/"))
            .map(|item| item.map_err(Error::from));

        staging
            .write()
            .map_err(|_| lockpoisoned())?
            .write_files_from_tar_stream(tar_stream)
            .await
    }

}


/// Helper fn for std::sync::PoisonError wrapping
/// because std::sync::PoisonError is not Send for <Inner>
fn lockpoisoned() -> Error {
    anyhow!("Lock poisoned")
}
