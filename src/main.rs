//
// Copyright (c) 2020-2021 science+computing ag and other contributors
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
    const_err,
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

extern crate log as logcrate;
#[macro_use]
extern crate diesel;

use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Result;
use clap::ArgMatches;
use logcrate::debug;
use walkdir::WalkDir;
use rand as _; // Required to make lints happy

mod cli;
mod commands;
mod config;
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

#[tokio::main]
async fn main() -> Result<()> {
    let _ = env_logger::try_init()?;
    debug!("Debugging enabled");

    let app = cli::cli();
    let cli = app.get_matches();

    let repo = git2::Repository::discover(PathBuf::from("."))?;
    let repo_path = repo
        .workdir()
        .ok_or_else(|| anyhow!("Not a repository with working directory. Cannot do my job!"))?;

    let mut config = ::config::Config::default();

    config
        .merge(::config::File::from(repo_path.join("config.toml")))?
        .merge(::config::Environment::with_prefix("BUTIDO"))?;

    let config = config.try_into::<NotValidatedConfiguration>()?.validate()?;

    let max_packages = count_pkg_files(&repo_path);
    let hide_bars = cli.is_present("hide_bars") || crate::util::stdout_is_pipe();
    let progressbars = ProgressBars::setup(
        config.progress_format().clone(),
        config.spinner_format().clone(),
        hide_bars,
    );

    let load_repo = || -> Result<Repository> {
        let bar = progressbars.bar();
        bar.set_length(max_packages);
        let repo = Repository::load(&repo_path, &bar)?;
        bar.finish_with_message("Repository loading finished");
        Ok(repo)
    };

    let db_connection_config = crate::db::parse_db_connection_config(&config, &cli);
    match cli.subcommand() {
        Some(("generate-completions", matches)) => generate_completions(matches)?,
        Some(("db", matches)) => crate::commands::db(db_connection_config, &config, matches)?,
        Some(("build", matches)) => {
            let conn = crate::db::establish_connection(db_connection_config)?;

            let repo = load_repo()?;

            crate::commands::build(
                &repo_path,
                matches,
                progressbars,
                conn,
                &config,
                repo,
                &repo_path,
                max_packages,
            )
            .await?
        }
        Some(("what-depends", matches)) => {
            let repo = load_repo()?;
            let bar = progressbars.bar();
            bar.set_length(max_packages);
            crate::commands::what_depends(matches, &config, repo).await?
        }

        Some(("dependencies-of", matches)) => {
            let repo = load_repo()?;
            let bar = progressbars.bar();
            bar.set_length(max_packages);
            crate::commands::dependencies_of(matches, &config, repo).await?
        }

        Some(("versions-of", matches)) => {
            let repo = load_repo()?;
            crate::commands::versions_of(matches, repo).await?
        }

        Some(("env-of", matches)) => {
            let repo = load_repo()?;
            crate::commands::env_of(matches, repo).await?
        }

        Some(("find-pkg", matches)) => {
            let repo = load_repo()?;
            crate::commands::find_pkg(matches, &config, repo).await?
        }

        Some(("source", matches)) => {
            let repo = load_repo()?;
            crate::commands::source(matches, &config, repo, progressbars).await?
        }

        Some(("release", matches)) => {
            crate::commands::release(db_connection_config, &config, matches).await?
        }

        Some(("lint", matches)) => {
            let repo = load_repo()?;
            crate::commands::lint(&repo_path, matches, progressbars, &config, repo).await?
        }

        Some(("tree-of", matches)) => {
            let repo = load_repo()?;
            crate::commands::tree_of(matches, repo, progressbars).await?
        }

        Some((other, _)) => return Err(anyhow!("Unknown subcommand: {}", other)),
        None => return Err(anyhow!("No subcommand")),
    }

    Ok(())
}

fn count_pkg_files(p: &Path) -> u64 {
    WalkDir::new(p)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|d| d.file_type().is_file())
        .filter(|f| {
            f.path()
                .file_name()
                .map(|name| name == "pkg.toml")
                .unwrap_or(false)
        })
        .count() as u64
}

fn generate_completions(matches: &ArgMatches) -> Result<()> {
    use clap_generate::generate;
    use clap_generate::generators::{Bash, Elvish, Fish, Zsh};

    let appname = "butido";
    match matches.value_of("shell").unwrap() {
        // unwrap safe by clap
        "bash" => {
            generate::<Bash, _>(&mut cli::cli(), appname, &mut std::io::stdout());
        }
        "elvish" => {
            generate::<Elvish, _>(&mut cli::cli(), appname, &mut std::io::stdout());
        }
        "fish" => {
            generate::<Fish, _>(&mut cli::cli(), appname, &mut std::io::stdout());
        }
        "zsh" => {
            generate::<Zsh, _>(&mut cli::cli(), appname, &mut std::io::stdout());
        }
        _ => unreachable!(),
    }

    Ok(())
}
