//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use clap::ArgMatches;
use log::{debug, info};
use itertools::Itertools;
use tokio_stream::StreamExt;

use crate::config::Configuration;
use crate::util::progress::ProgressBars;
use crate::endpoint::Endpoint;

pub async fn endpoint(matches: &ArgMatches, config: &Configuration, progress_generator: ProgressBars) -> Result<()> {
    let endpoint_names = matches
        .value_of("endpoint_name")
        .map(String::from)
        .map(|ep| vec![ep])
        .unwrap_or_else(|| {
            config.docker()
                .endpoints()
                .iter()
                .map(|ep| ep.name())
                .cloned()
                .collect()
        });

    match matches.subcommand() {
        Some(("ping", matches)) => ping(endpoint_names, matches, config, progress_generator).await,
        Some(("stats", matches)) => stats(endpoint_names, matches, config, progress_generator).await,
        Some(("container", matches)) => crate::commands::endpoint_container::container(endpoint_names, matches, config).await,
        Some(("containers", matches)) => containers(endpoint_names, matches, config).await,
        Some((other, _)) => Err(anyhow!("Unknown subcommand: {}", other)),
        None => Err(anyhow!("No subcommand")),
    }
}

async fn ping(endpoint_names: Vec<String>,
    matches: &ArgMatches,
    config: &Configuration,
    progress_generator: ProgressBars
) -> Result<()> {
    let n_pings = matches.value_of("ping_n").map(u64::from_str).transpose()?.unwrap(); // safe by clap
    let sleep = matches.value_of("ping_sleep").map(u64::from_str).transpose()?.unwrap(); // safe by clap
    let endpoints = connect_to_endpoints(config, &endpoint_names).await?;
    let multibar = Arc::new({
        let mp = indicatif::MultiProgress::new();
        if progress_generator.hide() {
            mp.set_draw_target(indicatif::ProgressDrawTarget::hidden());
        }
        mp
    });

    let ping_process = endpoints
        .iter()
        .map(|endpoint| {
            let bar = multibar.add(progress_generator.bar());
            bar.set_length(n_pings);
            bar.set_message(&format!("Pinging {}", endpoint.name()));

            async move {
                for i in 1..(n_pings + 1) {
                    debug!("Pinging {} for the {} time", endpoint.name(), i);
                    let r = endpoint.ping().await;
                    bar.inc(1);
                    if let Err(e) = r {
                        bar.finish_with_message(&format!("Pinging {} failed", endpoint.name()));
                        return Err(e)
                    }

                    tokio::time::sleep(tokio::time::Duration::from_secs(sleep)).await;
                }

                bar.finish_with_message(&format!("Pinging {} successful", endpoint.name()));
                Ok(())
            }
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<()>>();

    let multibar_block = tokio::task::spawn_blocking(move || multibar.join());
    tokio::join!(ping_process, multibar_block).0
}

async fn stats(endpoint_names: Vec<String>,
    matches: &ArgMatches,
    config: &Configuration,
    progress_generator: ProgressBars
) -> Result<()> {
    let csv = matches.is_present("csv");
    let endpoints = connect_to_endpoints(config, &endpoint_names).await?;
    let bar = progress_generator.bar();
    bar.set_length(endpoint_names.len() as u64);
    bar.set_message("Fetching stats");

    let hdr = crate::commands::util::mk_header([
        "Name",
        "Containers",
        "Images",
        "Kernel",
        "Memory",
        "Memory limit",
        "Cores",
        "OS",
        "System Time",
    ].to_vec());

    let data = endpoints
        .into_iter()
        .map(|endpoint| {
            let bar = bar.clone();
            async move {
                let r = endpoint.stats().await;
                bar.inc(1);
                r
            }
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<Vec<_>>>()
        .await
        .map_err(|e| {
            bar.finish_with_message("Fetching stats errored");
            e
        })?
        .into_iter()
        .map(|stat| {
            vec![
                stat.name,
                stat.containers.to_string(),
                stat.images.to_string(),
                stat.kernel_version,
                bytesize::ByteSize::b(stat.mem_total).to_string(),
                stat.memory_limit.to_string(),
                stat.n_cpu.to_string(),
                stat.operating_system.to_string(),
                stat.system_time.map(|t| t.to_string()).unwrap_or_else(|| String::from("unknown")),
            ]
        })
        .collect();

    bar.finish_with_message("Fetching stats successful");
    crate::commands::util::display_data(hdr, data, csv)
}


async fn containers(endpoint_names: Vec<String>,
    matches: &ArgMatches,
    config: &Configuration,
) -> Result<()> {
    match matches.subcommand() {
        Some(("list", matches)) => containers_list(endpoint_names, matches, config).await,
        Some((other, _)) => Err(anyhow!("Unknown subcommand: {}", other)),
        None => Err(anyhow!("No subcommand")),
    }
}

async fn containers_list(endpoint_names: Vec<String>,
    matches: &ArgMatches,
    config: &Configuration,
) -> Result<()> {
    let list_stopped = matches.is_present("list_stopped");
    let filter_image = matches.value_of("filter_image");
    let older_than_filter = matches.value_of("older_than")
        .map(humantime::parse_rfc3339_weak)
        .transpose()?
        .map(chrono::DateTime::<chrono::Local>::from);
    let newer_than_filter = matches.value_of("newer_than")
        .map(humantime::parse_rfc3339_weak)
        .transpose()?
        .map(chrono::DateTime::<chrono::Local>::from);
    let csv = matches.is_present("csv");
    let hdr = crate::commands::util::mk_header([
        "Endpoint",
        "Container id",
        "Image",
        "Created",
        "Status",
    ].to_vec());

    let data = connect_to_endpoints(config, &endpoint_names)
        .await?
        .into_iter()
        .map(|ep| async move {
            ep.container_stats().await.map(|stats| (ep.name().clone(), stats))
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<Vec<(_, _)>>>()
        .await?
        .into_iter()
        .map(|tpl| {
            let endpoint_name = tpl.0;
            tpl.1
                .into_iter()
                .filter(|stat| list_stopped || stat.state != "exited")
                .filter(|stat| filter_image.map(|fim| fim == stat.image).unwrap_or(true))
                .filter(|stat| older_than_filter.as_ref().map(|time| time > &stat.created).unwrap_or(true))
                .filter(|stat| newer_than_filter.as_ref().map(|time| time < &stat.created).unwrap_or(true))
                .map(|stat| {
                    vec![
                        endpoint_name.clone(),
                        stat.id,
                        stat.image,
                        stat.created.to_string(),
                        stat.status,
                    ]
                })
                .collect::<Vec<Vec<String>>>()
        })
        .flatten()
        .collect::<Vec<Vec<String>>>();

    crate::commands::util::display_data(hdr, data, csv)
}

/// Helper function to connect to all endpoints from the configuration, that appear (by name) in
/// the `endpoint_names` list
pub(super) async fn connect_to_endpoints(config: &Configuration, endpoint_names: &[String]) -> Result<Vec<Arc<Endpoint>>> {
    let endpoint_configurations = config
        .docker()
        .endpoints()
        .iter()
        .filter(|ep| endpoint_names.contains(ep.name()))
        .cloned()
        .map(|ep_cfg| {
            crate::endpoint::EndpointConfiguration::builder()
                .endpoint(ep_cfg)
                .required_images(config.docker().images().clone())
                .required_docker_versions(config.docker().docker_versions().clone())
                .required_docker_api_versions(config.docker().docker_api_versions().clone())
                .build()
        })
        .collect::<Vec<_>>();

    info!("Endpoint config build");
    info!("Connecting to {n} endpoints: {eps}",
        n = endpoint_configurations.len(),
        eps = endpoint_configurations.iter().map(|epc| epc.endpoint().name()).join(", "));

    crate::endpoint::util::setup_endpoints(endpoint_configurations).await
}
