use getset::Getters;
use typed_builder::TypedBuilder;
use anyhow::Result;

use crate::util::docker::ImageName;
use crate::endpoint::EndpointManager;

#[derive(Getters, TypedBuilder)]
pub struct EndpointManagerConfiguration {
    #[getset(get = "pub")]
    endpoint: crate::config::Endpoint,

    #[getset(get = "pub")]
    #[builder(default)]
    required_images: Vec<ImageName>,

    #[getset(get = "pub")]
    #[builder(default)]
    required_docker_versions: Option<Vec<String>>,

    #[getset(get = "pub")]
    #[builder(default)]
    required_docker_api_versions: Option<Vec<String>>,
}

impl EndpointManagerConfiguration {
    pub async fn connect(self) -> Result<EndpointManager> {
        EndpointManager::setup(self).await
    }
}

