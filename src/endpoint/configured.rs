use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use std::sync::RwLock;
use std::str::FromStr;
use std::path::PathBuf;

use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use getset::{Getters, CopyGetters};
use shiplift::ContainerOptions;
use shiplift::Docker;
use shiplift::ExecContainerOptions;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use typed_builder::TypedBuilder;

use crate::util::docker::ImageName;
use crate::endpoint::EndpointConfiguration;
use crate::job::RunnableJob;
use crate::job::JobResource;
use crate::log::LogItem;
use crate::filestore::StagingStore;

#[derive(Getters, CopyGetters, TypedBuilder)]
pub struct ConfiguredEndpoint {
    #[getset(get = "pub")]
    name: String,

    #[getset(get = "pub")]
    docker: Docker,

    #[getset(get_copy = "pub")]
    speed: usize,

    #[getset(get_copy = "pub")]
    num_current_jobs: usize,

    #[getset(get_copy = "pub")]
    num_max_jobs: usize,
}

impl Debug for ConfiguredEndpoint {
    fn fmt(&self, f: &mut Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f,
            "ConfiguredEndpoint({}, {}/{})",
            self.name,
            self.num_current_jobs,
            self.num_max_jobs)
    }
}

impl ConfiguredEndpoint {
    pub(in super) async fn setup(epc: EndpointConfiguration) -> Result<Self> {
        let ep = ConfiguredEndpoint::setup_endpoint(epc.endpoint())?;

        let versions_compat     = ConfiguredEndpoint::check_version_compat(epc.required_docker_versions().as_ref(), &ep);
        let api_versions_compat = ConfiguredEndpoint::check_api_version_compat(epc.required_docker_api_versions().as_ref(), &ep);
        let imgs_avail          = ConfiguredEndpoint::check_images_available(epc.required_images().as_ref(), &ep);

        let (versions_compat, api_versions_compat, imgs_avail) =
            tokio::join!(versions_compat, api_versions_compat, imgs_avail);

        let _ = versions_compat?;
        let _ = api_versions_compat?;
        let _ = imgs_avail?;

        Ok(ep)
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

    pub async fn run_job(&self, job: RunnableJob, logsink: UnboundedSender<LogItem>, staging: Arc<RwLock<StagingStore>>) -> Result<Vec<PathBuf>>  {
        use crate::log::buffer_stream_to_line_stream;
        use tokio::stream::StreamExt;

        let (container_id, _warnings) = {
            let envs: Vec<String> = job.resources()
                .iter()
                .filter_map(|r| match r {
                    JobResource::Environment(k, v) => Some(format!("{}={}", k, v)),
                    JobResource::Artifact(_)       => None,
                })
                .collect();

            let builder_opts = shiplift::ContainerOptions::builder(job.image().as_ref())
                    .env(envs.iter().map(AsRef::as_ref).collect())
                    .build();

            let create_info = self.docker
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

        let container = self.docker.containers().get(&container_id);
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
            .map_err(|_| anyhow!("Lock poisoned"))?
            .write_files_from_tar_stream(tar_stream)
            .await
    }


}

