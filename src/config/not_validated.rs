use std::path::PathBuf;
use anyhow::anyhow;
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
    log_dir: PathBuf,

    #[serde(default = "default_progress_format")]
    #[getset(get = "pub")]
    progress_format: String,

    #[serde(default = "default_package_print_format")]
    #[getset(get = "pub")]
    package_print_format: String,

    #[getset(get = "pub")]
    script_highlight_theme: Option<String>,

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
        if let Some(configured_theme) = self.script_highlight_theme.as_ref() {
            let allowed_theme_present = [
                "base16-ocean.dark",
                "base16-eighties.dark",
                "base16-mocha.dark",
                "base16-ocean.light",
                "InspiredGitHub",
                "Solarized (dark)",
                "Solarized (light)",
            ].into_iter().any(|allowed_theme| configured_theme == *allowed_theme);

            if !allowed_theme_present {
                return Err(anyhow!("Theme not known: {}", configured_theme))
            }
        }

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

