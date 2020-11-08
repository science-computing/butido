use std::collections::BTreeMap;
use std::fmt::Debug;
use std::ops::Deref;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use getset::CopyGetters;
use getset::Getters;
use handlebars::Handlebars;
use serde::Deserialize;

use crate::phase::PhaseName;
use crate::util::EnvironmentVariableName;
use crate::util::docker::ImageName;

#[derive(Debug, Getters, Deserialize)]
pub struct NotValidatedConfiguration {
    #[getset(get = "pub")]
    repository: PathBuf,

    #[serde(default = "default_progress_format")]
    #[getset(get = "pub")]
    progress_format: String,

    #[serde(rename = "releases")]
    releases_directory: String,

    #[serde(rename = "staging")]
    staging_directory: String,

    #[getset(get = "pub")]
    #[serde(rename = "database_host")]
    database_host: String,

    #[getset(get = "pub")]
    #[serde(rename = "database_port")]
    database_port: String,

    #[getset(get = "pub")]
    #[serde(rename = "database_user")]
    database_user: String,

    #[getset(get = "pub")]
    #[serde(rename = "database_password")]
    database_password: String,

    #[getset(get = "pub")]
    #[serde(rename = "database_name")]
    database_name: String,

    #[getset(get = "pub")]
    docker: DockerConfig,

    #[getset(get = "pub")]
    containers: ContainerConfig,

    #[getset(get = "pub")]
    available_phases: Vec<PhaseName>,
}

impl<'reg> NotValidatedConfiguration {
    pub fn validate(self) -> Result<Configuration<'reg>> {
        // TODO: Implement proper validation

        let hb = {
            let mut hb = Handlebars::new();
            hb.register_template_string("releases", &self.releases_directory)?;
            hb.register_template_string("staging", &self.staging_directory)?;
            hb
        };

        Ok(Configuration {
            inner: self,
            hb,
        })
    }
}

#[derive(Debug)]
pub struct Configuration<'reg> {
    inner: NotValidatedConfiguration,
    hb: Handlebars<'reg>,
}

impl<'reg> Deref for Configuration<'reg> {
    type Target = NotValidatedConfiguration;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'reg> Configuration<'reg> {
    /// Get the path to the releases directory, interpolate every variable used in the config
    pub fn releases_directory(&self, hm: &BTreeMap<String, String>) -> Result<PathBuf> {
        self.hb.render("releases", hm)
            .map(PathBuf::from)
            .context("Interpolating variables into 'release' setting from configuration")
    }

    /// Get the path to the staging directory, interpolate every variable used in the config
    pub fn staging_directory(&self, hm: &BTreeMap<String, String>) -> Result<PathBuf> {
        self.hb.render("staging", hm)
            .map(PathBuf::from)
            .context("Interpolating variables into 'staging' setting from configuration")
    }
}


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

#[derive(Debug, Getters, Deserialize)]
pub struct ContainerConfig {
    #[getset(get = "pub")]
    allowed_env: Vec<EnvironmentVariableName>,
}


#[derive(Clone, Debug, Getters, CopyGetters, Deserialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum EndpointType {
    Socket,
    Http,
}


fn default_progress_format() -> String {
    String::from("[{elapsed_precise}] ({percent:>3}%): {bar:40.cyan/blue} | {msg}")
}
