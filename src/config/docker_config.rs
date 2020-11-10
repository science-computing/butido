use getset::{CopyGetters, Getters};
use serde::Deserialize;

use crate::config::Endpoint;
use crate::util::docker::ImageName;

#[derive(Debug, Getters, CopyGetters, Deserialize)]
pub struct DockerConfig {
    /// The required docker version
    ///
    /// If not set, it will not be checked, which might result in weird things?
    ///
    /// # Note
    ///
    /// Because the docker API returns strings, not a version object, each compatible version must
    /// be listed.
    #[getset(get = "pub")]
    docker_versions: Option<Vec<String>>,

    /// The required docker api version
    ///
    /// If not set, it will not be checked, which might result in weird things?
    ///
    /// # Note
    ///
    /// Because the docker API returns strings, not a version object, each compatible version must
    /// be listed.
    #[getset(get = "pub")]
    docker_api_versions: Option<Vec<String>>,

    /// Whether the program should verify that the required images are present.
    /// You want this to be true normally.
    #[getset(get_copy = "pub")]
    verify_images_present: bool,

    #[getset(get = "pub")]
    images: Vec<ImageName>,

    #[getset(get = "pub")]
    endpoints: Vec<Endpoint>,
}

