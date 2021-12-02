//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use getset::Getters;
use serde::Deserialize;
use std::path::PathBuf;

use crate::config::util::*;
use crate::config::Configuration;
use crate::config::ContainerConfig;
use crate::config::DockerConfig;
use crate::package::PhaseName;

/// The configuration that is loaded from the filesystem
#[derive(Debug, Getters, Deserialize)]
pub struct NotValidatedConfiguration {

    /// Compatibility setting
    ///
    /// If the version of butido is (semver) incompatible to this setting in the configuration,
    /// butido won't execute any further because it might fail later due to configuration
    /// incompatibilities
    #[getset(get = "pub")]
    compatibility: semver::VersionReq,

    /// The directory logs are written to, if logs are requested in plaintext files
    #[getset(get = "pub")]
    log_dir: PathBuf,

    /// Whether the script interpolation feature should be struct, i.e. missing variables result in
    /// a failing interpolation. This should be `true` for most users.
    #[serde(default = "default_strict_script_interpolation")]
    #[getset(get = "pub")]
    strict_script_interpolation: bool,

    /// The format of the progress bars
    #[serde(default = "default_progress_format")]
    #[getset(get = "pub")]
    progress_format: String,

    /// The format of the spinners in the CLI
    #[serde(default = "default_spinner_format")]
    #[getset(get = "pub")]
    spinner_format: String,

    /// The format used to print a package
    ///
    /// This is handlebars syntax
    #[serde(default = "default_package_print_format")]
    #[getset(get = "pub")]
    package_print_format: String,

    /// How many lines should be printed from the log if a build fails
    #[serde(default = "default_build_error_lines")]
    #[getset(get = "pub")]
    build_error_lines: usize,

    /// The theme used to highlight scripts when printing them to the CLI
    #[getset(get = "pub")]
    script_highlight_theme: Option<String>,

    /// The linter executable that is used to lint packaging scripts
    #[getset(get = "pub")]
    script_linter: Option<PathBuf>,

    /// The shebang that is added at the very beginning of the package scripts
    #[serde(default = "default_script_shebang")]
    #[getset(get = "pub")]
    shebang: String,

    /// The directory where releases are stored
    #[serde(rename = "releases_root")]
    #[getset(get = "pub")]
    releases_directory: PathBuf,

    /// The names of the directories inside the `releases_directory` to store different releases in
    #[serde(rename = "release_stores")]
    #[getset(get = "pub")]
    release_stores: Vec<String>,

    /// The directory where intermediate ("staging") artifacts are stored.
    /// This is used as a root directory, a UUID-named directory will be added below this, using
    /// the UUID of the submit
    #[serde(rename = "staging")]
    #[getset(get = "pub")]
    staging_directory: PathBuf,

    /// Where the sources are cached
    #[serde(rename = "source_cache")]
    #[getset(get = "pub")]
    source_cache_root: PathBuf,

    /// The hostname used to connect to the database
    #[getset(get = "pub")]
    #[serde(rename = "database_host")]
    database_host: String,

    /// The post used to connect to the database
    #[getset(get = "pub")]
    #[serde(rename = "database_port")]
    database_port: u16,

    /// The user used to connect to the database
    #[getset(get = "pub")]
    #[serde(rename = "database_user")]
    database_user: String,

    /// The password used to connect to the database
    #[getset(get = "pub")]
    #[serde(rename = "database_password")]
    database_password: String,

    /// The name of the database
    #[getset(get = "pub")]
    #[serde(rename = "database_name")]
    database_name: String,

    /// The configuration for the docker endpoints
    #[getset(get = "pub")]
    #[serde(rename = "database_connection_timeout")]
    database_connection_timeout: Option<u16>,

    #[getset(get = "pub")]
    docker: DockerConfig,

    /// The configuration for the containers
    #[getset(get = "pub")]
    containers: ContainerConfig,

    /// The names of the phases which should be compiled into the packaging script
    #[getset(get = "pub")]
    available_phases: Vec<PhaseName>,
}

impl NotValidatedConfiguration {
    /// Validate the NotValidatedConfiguration object and make it into a Configuration object, if
    /// validation succeeds
    ///
    /// This function does sanity-checking on the configuration values.
    /// It fails with the appropriate error message if a setting is bogus.
    pub fn validate(self) -> Result<Configuration> {
        let crate_version = semver::Version::parse(env!("CARGO_PKG_VERSION"))
            .context("Parsing version of crate (CARGO_PKG_VERSION) into semver::Version object")?;

        if !self.compatibility.matches(&crate_version) {
            return Err(anyhow!(
                "Configuration is not compatible to butido {}",
                crate_version
            ));
        }

        // Error if staging_directory is not a directory
        if !self.staging_directory.is_dir() {
            return Err(anyhow!(
                "Not a directory: staging = {}",
                self.staging_directory.display()
            ));
        }

        // Error if releases_directory is not a directory
        if !self.releases_directory.is_dir() {
            return Err(anyhow!(
                "Not a directory: releases = {}",
                self.releases_directory.display()
            ));
        }

        if self.release_stores.is_empty() {
            return Err(anyhow!("You need at least one release store in 'release_stores'"))
        }

        // Error if source_cache_root is not a directory
        if !self.source_cache_root.is_dir() {
            return Err(anyhow!(
                "Not a directory: releases = {}",
                self.source_cache_root.display()
            ));
        }

        // Error if there are no phases configured
        if self.available_phases.is_empty() {
            return Err(anyhow!("No phases configured"));
        }

        // Error if script highlighting theme is not valid
        if let Some(configured_theme) = self.script_highlight_theme.as_ref() {
            let allowed_theme_present = [
                // from syntect
                "base16-ocean.dark",
                "base16-eighties.dark",
                "base16-mocha.dark",
                "base16-ocean.light",
                "InspiredGitHub",
                "Solarized (dark)",
                "Solarized (light)",
            ]
            .iter()
            .any(|allowed_theme| configured_theme == *allowed_theme);

            if !allowed_theme_present {
                return Err(anyhow!("Theme not known: {}", configured_theme));
            }
        }

        Ok(Configuration { inner: self })
    }
}
