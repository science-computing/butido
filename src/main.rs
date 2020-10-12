#[macro_use] extern crate log;

use std::path::Path;
use std::path::PathBuf;
use anyhow::Result;
use walkdir::WalkDir;

mod cli;
mod util;
mod package;
mod phase;
mod config;
mod repository;
use crate::config::DockerConfig;
use crate::repository::Repository;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::cli();
    let cli = cli.get_matches();

    let mut config = ::config::Config::default();
    config
        .merge(::config::File::with_name("config"))?
        .merge(::config::Environment::with_prefix("YABOS"))?;
        // Add in settings from the environment (with a prefix of YABOS)
        // Eg.. `YABOS_DEBUG=1 ./target/app` would set the `debug` key
    //

    let docker_config = config.get::<DockerConfig>("docker")?;

    let repo_path = PathBuf::from(config.get_str("repository")?);
    let progress  = indicatif::ProgressBar::new(count_pkg_files(&repo_path));
    progress.set_style(indicatif::ProgressStyle::default_bar());
    let repo      = Repository::load(&repo_path, &progress)?;
    progress.finish_with_message("Repository loading finished");

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
