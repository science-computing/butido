use std::path::PathBuf;
use anyhow::anyhow;
use anyhow::Result;
use getset::Getters;
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

    #[serde(default = "default_strict_script_interpolation")]
    #[getset(get = "pub")]
    strict_script_interpolation: bool,

    #[serde(default = "default_progress_format")]
    #[getset(get = "pub")]
    progress_format: String,

    #[serde(default = "default_package_print_format")]
    #[getset(get = "pub")]
    package_print_format: String,

    #[getset(get = "pub")]
    script_highlight_theme: Option<String>,

    #[serde(rename = "releases")]
    #[getset(get = "pub")]
    releases_directory: PathBuf,

    #[serde(rename = "staging")]
    #[getset(get = "pub")]
    staging_directory: PathBuf,

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

impl NotValidatedConfiguration {
    pub fn validate(self) -> Result<Configuration> {
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
            ].iter().any(|allowed_theme| configured_theme == *allowed_theme);

            if !allowed_theme_present {
                return Err(anyhow!("Theme not known: {}", configured_theme))
            }
        }

        Ok(Configuration { inner: self })
    }
}

