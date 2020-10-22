use std::sync::Arc;
use std::sync::RwLock;
use std::str::FromStr;

use anyhow::anyhow;
use anyhow::Result;
use anyhow::Error;
use tokio::sync::mpsc::UnboundedSender;

use crate::util::docker::ImageName;
use crate::endpoint::configured::ConfiguredEndpoint;
use crate::endpoint::managerconf::EndpointManagerConfiguration;

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

    pub async fn run_job(self, job: RunnableJob, logsink: UnboundedSender<LogItem>) -> Result<()>  {
        unimplemented!()
    }

}

type LogItem     = (); // TODO Replace with actual implementation
type RunnableJob = (); // TODO Replace with actual implementation


/// Helper fn for std::sync::PoisonError wrapping
/// because std::sync::PoisonError is not Send for <Inner>
fn lockpoisoned() -> Error {
    anyhow!("Lock poisoned")
}
