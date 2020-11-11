use std::path::PathBuf;
use anyhow::Result;
use getset::Getters;
use handlebars::Handlebars;
use serde::Deserialize;

use crate::config::Configuration;
use crate::config::ContainerConfig;
use crate::config::DockerConfig;
use crate::config::util::*;
use crate::phase::PhaseName;

#[derive(Debug, Getters, Deserialize)]
pub struct NotValidatedConfiguration {
    #[getset(get = "pub")]
    repository: PathBuf,

    #[serde(default = "default_progress_format")]
    #[getset(get = "pub")]
    progress_format: String,

    #[serde(default = "default_package_print_format")]
    #[getset(get = "pub")]
    package_print_format: String,

    #[serde(rename = "releases")]
    releases_directory: String,

    #[serde(rename = "staging")]
    staging_directory: String,

    #[serde(rename = "source_cache")]
    #[getset(get = "pub")]
    source_cache_root: String,

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

