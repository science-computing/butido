#[macro_use] extern crate log;

use std::path::PathBuf;
use anyhow::Result;

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
    let repo      = Repository::load(&repo_path)?;

    Ok(())
}
