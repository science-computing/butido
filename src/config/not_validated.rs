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

// The configuration version must be increased each time breaking configuration changes are made
// (that require users to update their configurations) and the required changes must be documented
// in CHANGELOG.toml:
const CONFIGURATION_VERSION: u16 = 1;

/// The configuration that is loaded from the filesystem
#[derive(Debug, Getters, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotValidatedConfiguration {
    /// Compatibility setting to check if the butido configuration from the user is compatible with
    /// the current butido version (this is kinda optional since the configuration is type checked
    /// but it's useful to avoid accidents (butido will abort if the configuration isn't
    /// compatible) and to inform users of required changes).
    #[getset(get = "pub")]
    compatibility: u16,

    /// The directory logs are written to, if logs are requested in plaintext files
    #[getset(get = "pub")]
    log_dir: PathBuf,

    /// Whether the script interpolation feature should be strict, i.e. missing variables result in
    /// a failing interpolation. This should be `true` for most users.
    #[serde(default = "default_strict_script_interpolation")]
    #[getset(get = "pub")]
    strict_script_interpolation: bool,

    /// The format of the progress status output for various operations
    #[serde(default = "default_progress_format")]
    #[getset(get = "pub")]
    progress_format: String,

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

    /// The hostname/FQDN/IP used to connect to the database
    #[getset(get = "pub")]
    database_host: String,

    /// The port used to connect to the database
    #[getset(get = "pub")]
    database_port: u16,

    /// The user used to connect to the database
    #[getset(get = "pub")]
    database_user: String,

    /// The password used to connect to the database
    #[getset(get = "pub")]
    database_password: String,

    /// The name of the database
    #[getset(get = "pub")]
    database_name: String,

    /// The database connection timeout in seconds
    #[getset(get = "pub")]
    #[serde(default = "default_database_connection_timeout")]
    database_connection_timeout: u16,

    /// The default limit for database queries (when listing tables with the `db` subcommand;
    /// 0=unlimited (not recommended as it might result in OOM kills))
    #[serde(default = "default_database_query_limit")]
    #[getset(get = "pub")]
    database_default_query_limit: usize,

    /// The configuration for the Docker endpoints and images
    #[getset(get = "pub")]
    docker: DockerConfig,

    /// The configuration for the containers
    #[getset(get = "pub")]
    containers: ContainerConfig,

    /// The names of the phases which should be compiled into the packaging script
    #[getset(get = "pub")]
    available_phases: Vec<PhaseName>,
}

fn load_changelog() -> Result<std::collections::HashMap<String, String>> {
    let changelog_toml = include_str!("../../CHANGELOG.toml");
    // Ideally this would be done at compile time but we'll use tests for now to avoid unnecessary
    // runtime errors due to TOML (parsing) errors in CHANGELOG.toml:
    toml::from_str(changelog_toml).context("Butido bug: Couldn't parse the embedded CHANGELOG.toml")
}

// Helper function to check if the configuration should be compatible before loading (type checking) it:
pub fn check_compatibility(config: &config::Config) -> Result<()> {
    // We don't use config.get_int() as it is petty lax and, e.g., converts `true` to `1`:
    let compatibility = config.get_string("compatibility").context(
        "Make sure that the butido configuration is present and that \"compatibility\" is set",
    )?;
    // Parse the compatibility setting:
    let compatibility = compatibility
        .parse::<u16>()
        .with_context(|| {
            anyhow!("Failed to parse the value of the compatibility setting ({}) into a number (str -> u16)", compatibility)
        })
        .context("The format of the \"compatibility\" setting has changed from a string to a number")
        .context("Set \"compatibility\" to 0 to get a summary of the required changes")?;

    if compatibility == CONFIGURATION_VERSION {
        Ok(()) // Everything is fine
    } else {
        // The configuration is incompatible (too old or too new):
        let err = Err(anyhow!(
            "The provided configuration is not compatible with this butido binary"
        ))
        .with_context(|| {
            anyhow!(
                "The expected configuration version is {} while the provided configuration has a compatibility setting of {}",
                CONFIGURATION_VERSION,
                compatibility,
            )
        });

        if compatibility > CONFIGURATION_VERSION {
            err.context("This version of butido is too old for your configuration")
                .context("Update butido or downgrade your configuration")
        } else {
            // The configuration must be updated -> try to output the required changes from the changelog:
            let changelog = load_changelog().context(
                "Please refer to the changelog in README.{md,toml} for the required configuration changes",
            )?;

            // Output the required configuration changes to stderr:
            eprintln!(
                "The butido configuration is too old and the following changes are required:"
            );
            for i in (compatibility + 1)..=CONFIGURATION_VERSION {
                if let Some(changelog_entry) = changelog.get(&i.to_string()) {
                    eprintln!("- Version {i}: {changelog_entry}");
                } else {
                    eprintln!("- Version {i}: Error (butido bug): The changelog entry is missing!");
                }
            }
            eprintln!("- Update the `compatibility` setting to `{CONFIGURATION_VERSION}`\n");

            err
        }
    }
}

impl NotValidatedConfiguration {
    /// Validate the NotValidatedConfiguration object and make it into a Configuration object, if
    /// validation succeeds
    ///
    /// This function does sanity-checking on the configuration values.
    /// It fails with the appropriate error message if a setting is bogus.
    pub fn validate(self) -> Result<Configuration> {
        self.validate_config(false)
    }
    fn validate_config(self, skip_filesystem_checks: bool) -> Result<Configuration> {
        // A trivial helper to check if a directory is missing:
        let check_directory_exists = |path: &PathBuf, config_key_name: &str| -> Result<()> {
            if skip_filesystem_checks || path.is_dir() {
                Ok(())
            } else {
                Err(anyhow!(
                    "Not a directory: {} = {}",
                    config_key_name,
                    path.display()
                ))
            }
        };

        // Double-check the compatibility (mainly to avoid "error: field `compatibility` is never read")
        if self.compatibility != CONFIGURATION_VERSION {
            anyhow::bail!("The provided configuration is not compatible with this butido binary");
        }

        // Error if the configured directories are missing or no directories:
        check_directory_exists(&self.log_dir, "log_dir")?;
        check_directory_exists(&self.releases_directory, "releases_root")?;
        check_directory_exists(&self.staging_directory, "staging")?;
        check_directory_exists(&self.source_cache_root, "source_cache")?;

        if self.release_stores.is_empty() {
            anyhow::bail!("You need at least one release store in 'release_stores'");
        }

        // Error if there are no phases configured
        if self.available_phases.is_empty() {
            anyhow::bail!("No phases configured");
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
                anyhow::bail!("Theme not known: {}", configured_theme);
            }
        }

        Ok(Configuration { inner: self })
    }
}

#[cfg(test)]
mod tests {
    use super::check_compatibility;
    use super::load_changelog;
    use super::NotValidatedConfiguration;
    use super::CONFIGURATION_VERSION;

    use anyhow::Result;

    #[test]
    // A test to guard against unnecessary runtime failures
    fn test_loading_changelog_toml() {
        let changelog = load_changelog();
        assert!(changelog.is_ok());
        let changelog = changelog.unwrap();
        for i in 0..=CONFIGURATION_VERSION {
            assert!(changelog.contains_key(&i.to_string()));
        }
    }

    // A helper function to load and validate butido configuration files:
    fn test_loading_configuration_file(file_path: &str) -> Result<()> {
        let config = config::Config::builder()
            .add_source(config::File::with_name(file_path))
            .build()?;
        assert!(check_compatibility(&config).is_ok());
        let config = config.try_deserialize::<NotValidatedConfiguration>();
        assert!(config.is_ok(), "Config loading failed: {config:?}");
        let config = config.unwrap().validate_config(true);
        assert!(config.is_ok(), "Config validation failed: {config:?}");

        Ok(())
    }

    #[test]
    // A test to ensure the example configuration file is up-to-date and valid
    fn test_loading_example_configuration_file() -> Result<()> {
        test_loading_configuration_file("config.toml")?;
        Ok(())
    }

    #[test]
    // A test to ensure the example repo config file is up-to-date and valid
    fn test_loading_example_repo_configuration_file() -> Result<()> {
        test_loading_configuration_file("examples/packages/repo/config.toml")?;
        Ok(())
    }
}
