#[macro_use] extern crate log as logcrate;
#[macro_use] extern crate diesel;

use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use anyhow::anyhow;
use logcrate::debug;
use walkdir::WalkDir;

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
mod phase;
mod repository;
mod schema;
mod ui;
mod util;

use crate::config::*;
use crate::repository::Repository;
use crate::util::progress::ProgressBars;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = env_logger::try_init()?;
    debug!("Debugging enabled");

    let cli = cli::cli();
    let cli = cli.get_matches();

    let mut config = ::config::Config::default();
    config
        .merge(::config::File::with_name("config"))?
        .merge(::config::Environment::with_prefix("BUTIDO"))?;
        // Add in settings from the environment (with a prefix of YABOS)
        // Eg.. `YABOS_DEBUG=1 ./target/app` would set the `debug` key
    //

    let config: Configuration = config.try_into::<NotValidatedConfiguration>()?.validate()?;
    let repo_path             = PathBuf::from(config.repository());
    let _                     = crate::ui::package_repo_cleanness_check(&repo_path)?;
    let max_packages          = count_pkg_files(&repo_path);
    let progressbars          = ProgressBars::setup(config.progress_format().clone());

    let load_repo = || -> Result<Repository> {
        let bar = progressbars.repo_loading();
        bar.set_length(max_packages);
        let repo = Repository::load(&repo_path, &bar)?;
        bar.finish_with_message("Repository loading finished");
        Ok(repo)
    };

    let db_connection_config = crate::db::parse_db_connection_config(&config, &cli);
    match cli.subcommand() {
        ("db", Some(matches))           => db::interface(db_connection_config, matches)?,
        ("build", Some(matches))        => {
            let conn = crate::db::establish_connection(db_connection_config)?;

            let repo = load_repo()?;

            crate::commands::build(matches, progressbars, conn, &config, repo, &repo_path, max_packages as u64).await?
        },
        ("what-depends", Some(matches)) => {
            let repo = load_repo()?;
            let bar = progressbars.what_depends();
            bar.set_length(max_packages);
            crate::commands::what_depends(matches, &config, repo).await?
        },

        ("dependencies-of", Some(matches)) => {
            let repo = load_repo()?;
            let bar = progressbars.what_depends();
            bar.set_length(max_packages);
            crate::commands::dependencies_of(matches, &config, repo).await?
        },

        ("versions-of", Some(matches)) => {
            let repo = load_repo()?;
            crate::commands::versions_of(matches, repo).await?
        },

        ("env-of", Some(matches)) => {
            let repo = load_repo()?;
            crate::commands::env_of(matches, repo).await?
        }

        (other, _) => return Err(anyhow!("Unknown subcommand: {}", other)),
    }

    Ok(())
}

fn count_pkg_files(p: &Path) -> u64 {
    WalkDir::new(p)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|d| d.file_type().is_file())
        .filter(|f| f.path().file_name().map(|name| name == "pkg.toml").unwrap_or(false))
        .count() as u64
}

