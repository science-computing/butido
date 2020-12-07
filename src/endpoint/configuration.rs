use getset::Getters;
use typed_builder::TypedBuilder;

use crate::util::docker::ImageName;

#[derive(Getters, TypedBuilder)]
pub struct EndpointConfiguration {
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

