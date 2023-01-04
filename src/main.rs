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
    private_in_public,
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
    while_true,
)]
#![allow(macro_use_extern_crate)]
#![allow(unstable_name_collisions)] // TODO: Remove me with the next rustc update (probably)

extern crate log as logcrate;
#[macro_use]
extern crate diesel;

use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use logcrate::debug;
use logcrate::error;
use rand as _; // Required to make lints happy
use aquamarine as _; // doc-helper crate

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

pub const VERSION_LONG: &str = concatdoc!{"
    butido ", env!("VERGEN_GIT_DESCRIBE"), "
    Git SHA:              ", env!("VERGEN_GIT_SHA"), "
    Git Commit Timestamp: ", env!("VERGEN_GIT_COMMIT_TIMESTAMP"), "
    Build Timestamp:      ", env!("VERGEN_BUILD_TIMESTAMP"), "
    Debug Build:          ", env!("VERGEN_CARGO_DEBUG")
};

#[tokio::main]
async fn main() -> Result<()> {
    human_panic::setup_panic!(Metadata {
        name: env!("CARGO_PKG_NAME").into(),
        version: env!("CARGO_PKG_VERSION").into(),
        authors: "science-computing ag, opensoftware <opensoftware@science-computing.de>".into(),
        homepage: "atos.net/de/deutschland/sc".into(),
    });

    tracing_subscriber::fmt::fmt()
        .with_max_level(tracing::Level::WARN)
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();
    debug!("Debugging enabled");

    let app = cli::cli();
    let cli = app.get_matches();

    // check if the version flag is set
    if cli.get_flag("version") {
        println!("{VERSION_LONG}");
        std::process::exit(0);
    }

    let repo = git2::Repository::open(PathBuf::from("."))
        .map_err(|e| match e.code() {
            git2::ErrorCode::NotFound => {
                eprintln!("Butido must be executed in the top-level of the git repository");
                std::process::exit(1)
            },
            _ => Error::from(e),
        })?;

    let repo_path = repo
        .workdir()
        .ok_or_else(|| anyhow!("Not a repository with working directory. Cannot do my job!"))?;

    let mut config = ::config::Config::default();
    config.merge(::config::File::from(repo_path.join("config.toml")).required(true))
        .context("Failed to load config.toml from repository")?;

    {
        let xdg = xdg::BaseDirectories::with_prefix("butido")?;
        let xdg_config_file = xdg.find_config_file("config.toml");
        if let Some(xdg_config) = xdg_config_file {
            debug!("Configuration file found with XDG: {}", xdg_config.display());
            config.merge(::config::File::from(xdg_config).required(false))
                .context("Failed to load config.toml from XDG configuration directory")?;
        } else {
            debug!("No configuration file found with XDG: {}", xdg.get_config_home().display());
        }
    }

    config.merge(::config::Environment::with_prefix("BUTIDO"))?;

    let config = config.try_into::<NotValidatedConfiguration>()
        .context("Failed to load Configuration object")?
        .validate()
        .context("Failed to validate configuration")?;

    let hide_bars = cli.get_flag("hide_bars") || crate::util::stdout_is_pipe();
    let progressbars = ProgressBars::setup(
        config.progress_format().clone(),
        hide_bars,
    );

    let load_repo = || -> Result<Repository> {
        let bar = progressbars.bar()?;
        let repo = Repository::load(repo_path, &bar)
            .context("Loading the repository")?;
        bar.finish_with_message("Repository loading finished");
        Ok(repo)
    };

    let db_connection_config = crate::db::DbConnectionConfig::parse(&config, &cli)?;
    match cli.subcommand() {
        Some(("generate-completions", matches)) => generate_completions(matches),
        Some(("db", matches)) => crate::commands::db(db_connection_config, &config, matches)?,
        Some(("build", matches)) => {
            let conn = db_connection_config.establish_connection()?;

            let repo = load_repo()?;

            crate::commands::build(
                repo_path,
                matches,
                progressbars,
                conn,
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
            let conn = db_connection_config.establish_connection()?;
            crate::commands::find_artifact(matches, &config, progressbars, repo, conn)
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
            crate::commands::tree_of(matches, repo)
                .await
                .context("tree-of command failed")?
        }

        Some(("metrics", _)) => {
            let repo = load_repo()?;
            let conn = db_connection_config.establish_connection()?;
            crate::commands::metrics(repo_path, &config, repo, conn)
                .await
                .context("metrics command failed")?
        }

        Some(("endpoint", matches)) => {
            crate::commands::endpoint(matches, &config, progressbars)
                .await
                .context("endpoint command failed")?
        },
        Some((other, _)) => {
            error!("Unknown subcommand: {}", other);
            error!("Use --help to find available subcommands");
            return Err(anyhow!("Unknown subcommand: {}", other))
        },
        None => {
            error!("No subcommand.");
            error!("Use --help to find available subcommands");
            return Err(anyhow!("No subcommand"))
        },
    }

    Ok(())
}

fn generate_completions(matches: &ArgMatches) {
    use clap_complete::generate;
    use clap_complete::Shell;

    fn print_completions(shell: Shell, cmd: &mut clap::Command) {
        eprintln!("Generating shell completions for {shell}...");
        generate(shell, cmd, cmd.get_name().to_string(), &mut std::io::stdout());
    }

    // src/cli.rs enforces that `shell` is set to a valid `Shell` so this is always true:
    if let Some(shell) = matches.get_one::<Shell>("shell").copied() {
        print_completions(shell, &mut cli::cli());
    }
}
