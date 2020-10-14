use std::path::PathBuf;
use std::fmt::Debug;
use std::ops::Deref;

use anyhow::Result;
use getset::Getters;
use getset::CopyGetters;
use serde::Deserialize;

use crate::phase::PhaseName;
use crate::util::EnvironmentVariableName;
use crate::util::docker::ImageName;

#[derive(Debug, Getters, Deserialize)]
pub struct NotValidatedConfiguration {
    #[getset(get = "pub")]
    repository: PathBuf,

    #[getset(get = "pub")]
    docker: DockerConfig,

    #[getset(get = "pub")]
    containers: ContainerConfig,

    #[getset(get = "pub")]
    available_phases: Vec<PhaseName>,
}

impl NotValidatedConfiguration {
    pub fn validate(self) -> Result<Configuration> {
        Ok(Configuration(self)) // TODO: Implement properly
    }
}

#[derive(Debug)]
pub struct Configuration(NotValidatedConfiguration);

impl Deref for Configuration {
    type Target = NotValidatedConfiguration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


#[derive(Debug, Getters, Deserialize)]
pub struct DockerConfig {

    #[getset(get = "pub")]
    images: Vec<ImageName>,

    #[getset(get = "pub")]
    endpoints: Vec<Endpoint>,
}

#[derive(Debug, Getters, Deserialize)]
pub struct ContainerConfig {
    #[getset(get = "pub")]
    allowed_env: Vec<EnvironmentVariableName>,
}


#[derive(Debug, Getters, CopyGetters, Deserialize)]
pub struct Endpoint {
    #[getset(get = "pub")]
    name: String,

    #[getset(get = "pub")]
    uri: String,

    #[getset(get = "pub")]
    endpoint_type: EndpointType,

    /// Relative speed to other endpoints
    ///
    /// So if you have two servers, one with 12 cores and one with 24, you want to set "1" for the
    /// first and "2" for the second (or "12" for the first and "24" for the second - the ratio is
    /// the thing here)!
    #[getset(get_copy = "pub")]
    speed: usize,

    /// Maximum number of jobs which are allowed on this endpoint
    #[getset(get_copy = "pub")]
    maxjobs: usize,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub enum EndpointType {
    Socket,
    Http,
}


