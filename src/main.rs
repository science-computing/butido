#[macro_use] extern crate log;

use std::str::FromStr;
use std::path::PathBuf;
use anyhow::anyhow;
use anyhow::Result;
use anyhow::Error;

mod cli;
mod util;
mod package;
mod phase;
mod config;
use crate::config::DockerConfig;
use crate::config::Endpoint;
use crate::config::EndpointType;
use crate::package::Loader;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;

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

    let loader = Loader::new(PathBuf::from(config.get_str("repository")?));
    let p = cli.value_of("package")
        .map(String::from)
        .ok_or_else(|| anyhow!("BUG: Clap should enforce package parameter"))?;

    let package_name = PackageName::from(p);

    let p = loader.load(&package_name, &PackageVersionConstraint::Any)?
        .ok_or_else(|| anyhow!("No package found"))?;

    Ok(())
}
