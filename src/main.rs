#[macro_use] extern crate log;

use std::str::FromStr;
use anyhow::Result;
use anyhow::Error;

mod util;
mod package;
mod phase;
mod config;
use crate::config::DockerConfig;
use crate::config::Endpoint;
use crate::config::EndpointType;

#[tokio::main]
async fn main() -> Result<()> {
    let mut config = ::config::Config::default();
    config
        .merge(::config::File::with_name("config"))?
        .merge(::config::Environment::with_prefix("YABOS"))?;
        // Add in settings from the environment (with a prefix of YABOS)
        // Eg.. `YABOS_DEBUG=1 ./target/app` would set the `debug` key
    //

    let docker_config = config.get::<DockerConfig>("docker")?;

    let iter = docker_config
        .endpoints()
        .iter()
        .map(|ep| {
            match ep.endpoint_type() {
                EndpointType::Http => {
                    shiplift::Uri::from_str(ep.uri())
                        .map(|uri| shiplift::Docker::host(uri))
                        .map_err(Error::from)
                }

                EndpointType::Socket => {
                    Ok(shiplift::Docker::unix(ep.uri()))
                }
            }
        });

    for d in iter {
        let v = d?.version().await?;
        println!("Docker: {}", v.version);
        println!("API   : {}", v.api_version);
    }

    Ok(())
}
