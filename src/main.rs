//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

#![deny(
    anonymous_parameters,
    bad_style,
    dead_code,
    deprecated_in_future,
    explicit_outlives_requirements,
    improper_ctypes,
    keyword_idents,
    no_mangle_generic_items,
    non_ascii_idents,
    non_camel_case_types,
    non_shorthand_field_patterns,
    non_snake_case,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    trivial_numeric_casts,
    unconditional_recursion,
    unsafe_code,
    unstable_features,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_imports,
    unused_must_use,
    unused_mut,
    unused_parens,
    while_true
)]
#![allow(macro_use_extern_crate)]

#[macro_use]
extern crate diesel;

use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use aquamarine as _;
use clap::ArgMatches;
use rustversion as _; // This crate is (occasionally) required (e.g., when we need version specific Clippy overrides)
use tracing::{debug, error, warn};
use tracing_subscriber::layer::SubscriberExt;

mod cli;
mod commands;
mod config;
mod consts;
mod db;
mod endpoint;
mod filestore;
mod job;
mod log;
mod orchestrator;
mod package;
mod repository;
mod schema;
mod source;
mod ui;
mod util;

use crate::config::*;
use crate::repository::Repository;
use crate::util::progress::ProgressBars;
use indoc::concatdoc;

pub const VERSION_LONG: &str = concatdoc! {"
    butido ", env!("VERGEN_GIT_DESCRIBE"), "
    Git SHA:              ", env!("VERGEN_GIT_SHA"), "
    Git Commit Timestamp: ", env!("VERGEN_GIT_COMMIT_TIMESTAMP"), "
    Build Timestamp:      ", env!("VERGEN_BUILD_TIMESTAMP"), "
    Debug Build:          ", env!("VERGEN_CARGO_DEBUG")
};

#[tokio::main]
async fn main() -> Result<()> {
    human_panic::setup_panic!(human_panic::Metadata::new(
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    )
    .authors("science + computing AG, openSoftware <opensoftware@science-computing.de>")
    .homepage("https://github.com/science-computing/butido")
    .support("- Via https://github.com/science-computing/butido/issues or mail to opensoftware@science-computing.de"));

    let app = cli::cli();
    let cli = app.get_matches();

    let (chrome_layer, _guard) = match cli
        .get_flag("tracing-chrome")
        .then(|| tracing_chrome::ChromeLayerBuilder::new().build())
    {
        Some((chrome_layer, guard)) => (Some(chrome_layer), Some(guard)),
        _ => (None, None),
    };

    let subscriber = tracing_subscriber::fmt::fmt()
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::WARN.into())
                .from_env_lossy(),
        )
        .finish()
        .with(chrome_layer);

    tracing::subscriber::set_global_default(subscriber)?;
    debug!("Debugging enabled");

    // check if the version flag is set
    if cli.get_flag("version") {
        println!("{VERSION_LONG}");
        std::process::exit(0);
    }

    let repo = git2::Repository::open(PathBuf::from(".")).map_err(|e| match e.code() {
        git2::ErrorCode::NotFound => {
            eprintln!("Butido must be executed in the top-level of the Git repository");
            std::process::exit(1)
        }
        _ => Error::from(e),
    })?;

    let repo_path = repo
        .workdir()
        .ok_or_else(|| anyhow!("Not a repository with working directory. Cannot do my job!"))?;

    let mut config_builder =
        ::config::Config::builder().add_source(::config::File::from(repo_path.join("config.toml")));

    {
        let xdg = xdg::BaseDirectories::with_prefix("butido");
        let xdg_config_file_name = "config.toml";
        if let Some(xdg_config_file_path) = xdg.find_config_file(xdg_config_file_name) {
            debug!(
                "Configuration file found with XDG: {}",
                xdg_config_file_path.display()
            );
            config_builder = config_builder.add_source(::config::File::from(xdg_config_file_path));
        } else if let Some(xdg_config_home) = xdg.get_config_home() {
            // Returns the user-specific configuration directory (set by XDG_CONFIG_HOME or
            // default fallback, plus the prefix and profile if configured). Is guaranteed to
            // not return None unless no HOME could be found.
            debug!(
                "No configuration file found with XDG at the following path: {}",
                xdg_config_home.join(xdg_config_file_name).display()
            );
        } else {
            warn!("No HOME directory found! Cannot load the user specific butido configuration.");
        }
    }

    config_builder = config_builder.add_source(::config::Environment::with_prefix("BUTIDO"));

    let config = config_builder
        .build()
        .context("Failed to load and build the butido configuration")?;

    // Check the "compatibility" setting before loading (type checking) the configuration so that
    // we can better inform the users about required changes:
    check_compatibility(&config)
        .context("The butido configuration failed the compatibility check")?;

    let config = config
        .try_deserialize::<NotValidatedConfiguration>()
        .context("Failed to load (type check) the butido configuration")?
        .validate()
        .context("Failed to validate the butido configuration")?;

    let hide_bars = cli.get_flag("hide_bars") || crate::util::stdout_is_pipe();
    let progressbars = ProgressBars::setup(config.progress_format().clone(), hide_bars);

    let load_repo = || -> Result<Repository> {
        let bar = progressbars.bar()?;
        bar.set_message("Loading repository...");
        let repo = Repository::load(repo_path, &bar).context("Loading the repository")?;
        bar.finish_with_message("Repository loading finished");
        Ok(repo)
    };

    let db_connection_config = crate::db::DbConnectionConfig::parse(&config, &cli)?;
    match cli.subcommand() {
        Some(("generate-completions", matches)) => generate_completions(matches),
        Some(("db", matches)) => crate::commands::db(db_connection_config, &config, matches)?,
        Some(("build", matches)) => {
            let pool = db_connection_config.establish_pool()?;

            let repo = load_repo()?;

            crate::commands::build(
                repo_path,
                matches,
                progressbars,
                pool,
                &config,
                repo,
                repo_path,
            )
            .await
            .context("build command failed")?
        }
        Some(("what-depends", matches)) => {
            let repo = load_repo()?;
            crate::commands::what_depends(matches, &config, repo)
                .await
                .context("what-depends command failed")?
        }

        Some(("dependencies-of", matches)) => {
            let repo = load_repo()?;
            crate::commands::dependencies_of(matches, &config, repo)
                .await
                .context("dependencies-of command failed")?
        }

        Some(("versions-of", matches)) => {
            let repo = load_repo()?;
            crate::commands::versions_of(matches, repo)
                .await
                .context("versions-of command failed")?
        }

        Some(("env-of", matches)) => {
            let repo = load_repo()?;
            crate::commands::env_of(matches, repo)
                .await
                .context("env-of command failed")?
        }

        Some(("find-artifact", matches)) => {
            let repo = load_repo()?;
            let pool = db_connection_config.establish_pool()?;
            crate::commands::find_artifact(matches, &config, progressbars, repo, pool)
                .await
                .context("find-artifact command failed")?
        }

        Some(("find-pkg", matches)) => {
            let repo = load_repo()?;
            crate::commands::find_pkg(matches, &config, repo)
                .await
                .context("find-pkg command failed")?
        }

        Some(("source", matches)) => {
            let repo = load_repo()?;
            crate::commands::source(matches, &config, repo, progressbars)
                .await
                .context("source command failed")?
        }

        Some(("release", matches)) => {
            crate::commands::release(db_connection_config, &config, matches)
                .await
                .context("release command failed")?
        }

        Some(("lint", matches)) => {
            let repo = load_repo()?;
            crate::commands::lint(repo_path, matches, progressbars, &config, repo)
                .await
                .context("lint command failed")?
        }

        Some(("tree-of", matches)) => {
            let repo = load_repo()?;
            crate::commands::tree_of(matches, repo, &config)
                .await
                .context("tree-of command failed")?
        }

        Some(("metrics", _)) => {
            let repo = load_repo()?;
            let pool = db_connection_config.establish_pool()?;
            crate::commands::metrics(repo_path, &config, repo, pool)
                .await
                .context("metrics command failed")?
        }

        Some(("endpoint", matches)) => crate::commands::endpoint(matches, &config, progressbars)
            .await
            .context("endpoint command failed")?,
        Some((other, _)) => {
            error!("Unknown subcommand: {}", other);
            error!("Use --help to find available subcommands");
            anyhow::bail!("Unknown subcommand: {}", other);
        }
        None => {
            error!("No subcommand.");
            error!("Use --help to find available subcommands");
            anyhow::bail!("No subcommand");
        }
    }

    Ok(())
}

fn generate_completions(matches: &ArgMatches) {
    use clap_complete::generate;
    use clap_complete::Shell;

    fn print_completions(shell: Shell, cmd: &mut clap::Command) {
        eprintln!("Generating shell completions for {shell}...");
        generate(
            shell,
            cmd,
            cmd.get_name().to_string(),
            &mut std::io::stdout(),
        );
    }

    // src/cli.rs enforces that `shell` is set to a valid `Shell` so this is always true:
    if let Some(shell) = matches.get_one::<Shell>("shell").copied() {
        print_completions(shell, &mut cli::cli());
    }
}
