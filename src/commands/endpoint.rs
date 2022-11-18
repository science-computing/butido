//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'endpoint' subcommand

use std::collections::HashMap;
use std::io::Write;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Error;
use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use clap::ArgMatches;
use log::{debug, info, trace};
use itertools::Itertools;
use tokio_stream::StreamExt;

use crate::config::Configuration;
use crate::config::EndpointName;
use crate::util::progress::ProgressBars;
use crate::endpoint::Endpoint;

pub async fn endpoint(matches: &ArgMatches, config: &Configuration, progress_generator: ProgressBars) -> Result<()> {
    let endpoint_names = matches
        .value_of("endpoint_name")
        .map(String::from)
        .map(EndpointName::from)
        .map(|ep| vec![ep])
        .unwrap_or_else(|| {
            config.docker()
                .endpoints()
                .iter()
                .map(|(ep_name, _)| ep_name)
                .cloned()
                .collect()
        });

    match matches.subcommand() {
        Some(("ping", matches)) => ping(endpoint_names, matches, config, progress_generator).await,
        Some(("stats", matches)) => stats(endpoint_names, matches, config, progress_generator).await,
        Some(("container", matches)) => crate::commands::endpoint_container::container(endpoint_names, matches, config).await,
        Some(("containers", matches)) => containers(endpoint_names, matches, config).await,
        Some(("images", matches)) => images(endpoint_names, matches, config).await,
        Some((other, _)) => Err(anyhow!("Unknown subcommand: {}", other)),
        None => Err(anyhow!("No subcommand")),
    }
}

async fn ping(endpoint_names: Vec<EndpointName>,
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

    endpoints
        .iter()
        .map(|endpoint| {
            let bar = progress_generator.bar().map(|bar| {
                bar.set_length(n_pings);
                bar.set_message(format!("Pinging {}", endpoint.name()));
                multibar.add(bar.clone());
                bar
            });

            async move {
                let bar = bar?;
                for i in 1..(n_pings + 1) {
                    debug!("Pinging {} for the {} time", endpoint.name(), i);
                    let r = endpoint.ping().await;
                    bar.inc(1);
                    if let Err(e) = r {
                        bar.finish_with_message(format!("Pinging {} failed", endpoint.name()));
                        return Err(e)
                    }

                    tokio::time::sleep(tokio::time::Duration::from_secs(sleep)).await;
                }

                bar.finish_with_message(format!("Pinging {} successful", endpoint.name()));
                Ok(())
            }
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<()>>()
        .await
}

async fn stats(endpoint_names: Vec<EndpointName>,
    matches: &ArgMatches,
    config: &Configuration,
    progress_generator: ProgressBars
) -> Result<()> {
    let csv = matches.is_present("csv");
    let endpoints = connect_to_endpoints(config, &endpoint_names).await?;
    let bar = progress_generator.bar()?;
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
                stat.system_time.unwrap_or_else(|| String::from("unknown")),
            ]
        })
        .collect();

    bar.finish_with_message("Fetching stats successful");
    crate::commands::util::display_data(hdr, data, csv)
}


async fn containers(endpoint_names: Vec<EndpointName>,
    matches: &ArgMatches,
    config: &Configuration,
) -> Result<()> {
    match matches.subcommand() {
        Some(("list", matches)) => containers_list(endpoint_names, matches, config).await,
        Some(("prune", matches)) => containers_prune(endpoint_names, matches, config).await,
        Some(("top", matches)) => containers_top(endpoint_names, matches, config).await,
        Some(("stop", matches)) => containers_stop(endpoint_names, matches, config).await,
        Some((other, _)) => Err(anyhow!("Unknown subcommand: {}", other)),
        None => Err(anyhow!("No subcommand")),
    }
}

async fn containers_list(endpoint_names: Vec<EndpointName>,
    matches: &ArgMatches,
    config: &Configuration,
) -> Result<()> {
    let list_stopped = matches.is_present("list_stopped");
    let filter_image = matches.value_of("filter_image");
    let older_than_filter = crate::commands::util::get_date_filter("older_than", matches)?;
    let newer_than_filter = crate::commands::util::get_date_filter("newer_than", matches)?;
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
        .flat_map(|tpl| {
            let endpoint_name = tpl.0;
            tpl.1
                .into_iter()
                .filter(|stat| list_stopped || stat.state != "exited")
                .filter(|stat| filter_image.map(|fim| fim == stat.image).unwrap_or(true))
                .filter(|stat| older_than_filter.as_ref().map(|time| time > &stat.created).unwrap_or(true))
                .filter(|stat| newer_than_filter.as_ref().map(|time| time < &stat.created).unwrap_or(true))
                .map(|stat| {
                    vec![
                        endpoint_name.as_ref().to_owned(),
                        stat.id,
                        stat.image,
                        stat.created.to_string(),
                        stat.status,
                    ]
                })
                .collect::<Vec<Vec<String>>>()
        })
        .collect::<Vec<Vec<String>>>();

    crate::commands::util::display_data(hdr, data, csv)
}

async fn containers_prune(endpoint_names: Vec<EndpointName>,
    matches: &ArgMatches,
    config: &Configuration,
) -> Result<()> {
    let older_than_filter = crate::commands::util::get_date_filter("older_than", matches)?;
    let newer_than_filter = crate::commands::util::get_date_filter("newer_than", matches)?;

    let stats = connect_to_endpoints(config, &endpoint_names)
        .await?
        .into_iter()
        .map(move |ep| async move {
            let stats = ep.container_stats()
                .await?
                .into_iter()
                .filter(|stat| stat.state == "exited")
                .filter(|stat| older_than_filter.as_ref().map(|time| time > &stat.created).unwrap_or(true))
                .filter(|stat| newer_than_filter.as_ref().map(|time| time < &stat.created).unwrap_or(true))
                .map(|stat| (ep.clone(), stat))
                .collect::<Vec<(_, _)>>();
            Ok(stats)
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<Vec<_>>>()
        .await?;

    let prompt = format!("Really delete {} Containers?", stats.iter().flatten().count());
    if !dialoguer::Confirm::new().with_prompt(prompt).interact()? {
        return Ok(())
    }

    stats.into_iter()
        .flat_map(Vec::into_iter)
        .map(|(ep, stat)| async move {
            ep.get_container_by_id(&stat.id)
                .await?
                .ok_or_else(|| anyhow!("Failed to find existing container {}", stat.id))?
                .delete()
                .await
                .map_err(Error::from)
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<()>>()
        .await
}

async fn containers_top(endpoint_names: Vec<EndpointName>,
    matches: &ArgMatches,
    config: &Configuration,
) -> Result<()> {
    let limit = matches.value_of("limit").map(usize::from_str).transpose()?;
    let older_than_filter = crate::commands::util::get_date_filter("older_than", matches)?;
    let newer_than_filter = crate::commands::util::get_date_filter("newer_than", matches)?;
    let csv = matches.is_present("csv");

    let data = connect_to_endpoints(config, &endpoint_names)
        .await?
        .into_iter()
        .inspect(|ep| trace!("Fetching stats for endpoint: {}", ep.name()))
        .map(move |ep| async move {
            let stats = ep.container_stats()
                .await?
                .into_iter()
                .inspect(|stat| trace!("Fetching stats for container: {}", stat.id))
                .filter(|stat| stat.state == "running")
                .filter(|stat| older_than_filter.as_ref().map(|time| time > &stat.created).unwrap_or(true))
                .filter(|stat| newer_than_filter.as_ref().map(|time| time < &stat.created).unwrap_or(true))
                .map(|stat| (ep.clone(), stat))
                .collect::<Vec<(_, _)>>();
            Ok(stats)
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<Vec<_>>>()
        .await?
        .into_iter()
        .flat_map(Vec::into_iter)
        .inspect(|(_ep, stat)| trace!("Fetching container: {}", stat.id))
        .map(|(ep, stat)| async move {
            ep.get_container_by_id(&stat.id)
                .await?
                .ok_or_else(|| anyhow!("Failed to find existing container {}", stat.id))?
                .top(None)
                .await
                .with_context(|| anyhow!("Fetching 'top' for {}", stat.id))
                .map_err(Error::from)
                .map(|top| (stat.id, top))
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<Vec<_>>>()
        .await?
        .into_iter()
        .inspect(|(cid, _top)| trace!("Processing top of container: {}", cid))
        .map(|(container_id, top)| {
            let processes = if let Some(limit) = limit {
                top.processes.into_iter().take(limit).collect()
            } else {
                top.processes
            };

            let hm = top.titles
                .into_iter()
                .zip(processes.into_iter())
                .collect::<HashMap<String, Vec<String>>>();
            (container_id, hm)
        })
        .collect::<HashMap<String, HashMap<String, Vec<String>>>>();

    let hdr = crate::commands::util::mk_header({
        std::iter::once("Container ID")
            .chain({
                data.values()
                    .flat_map(|hm| hm.keys())
                    .map(|s| s.deref())
            })
            .collect::<Vec<&str>>()
            .into_iter()
            .unique()
            .collect()
    });

    let data = data.into_iter()
        .flat_map(|(container_id, top_hm)| {
            top_hm.values()
                .map(|t| std::iter::once(container_id.clone()).chain(t.iter().map(String::clone)).collect())
                .collect::<Vec<Vec<String>>>()
        })

        // ugly hack to bring order to the galaxy
        .sorted_by(|v1, v2| if let (Some(f1), Some(f2)) = (v1.iter().next(), v2.iter().next()) {
            f1.cmp(f2)
        } else {
            std::cmp::Ordering::Less
        })
        .collect::<Vec<Vec<String>>>();

    crate::commands::util::display_data(hdr, data, csv)
}


async fn containers_stop(endpoint_names: Vec<EndpointName>,
    matches: &ArgMatches,
    config: &Configuration,
) -> Result<()> {
    let older_than_filter = crate::commands::util::get_date_filter("older_than", matches)?;
    let newer_than_filter = crate::commands::util::get_date_filter("newer_than", matches)?;

    let stop_timeout = matches.value_of("timeout")
        .map(u64::from_str)
        .transpose()?
        .map(std::time::Duration::from_secs);

    let stats = connect_to_endpoints(config, &endpoint_names)
        .await?
        .into_iter()
        .map(move |ep| async move {
            let stats = ep.container_stats()
                .await?
                .into_iter()
                .filter(|stat| stat.state == "exited")
                .filter(|stat| older_than_filter.as_ref().map(|time| time > &stat.created).unwrap_or(true))
                .filter(|stat| newer_than_filter.as_ref().map(|time| time < &stat.created).unwrap_or(true))
                .map(|stat| (ep.clone(), stat))
                .collect::<Vec<(_, _)>>();
            Ok(stats)
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<Vec<_>>>()
        .await?;

    let prompt = format!("Really stop {} Containers?", stats.iter().flatten().count());
    if !dialoguer::Confirm::new().with_prompt(prompt).interact()? {
        return Ok(())
    }

    stats.into_iter()
        .flat_map(Vec::into_iter)
        .map(|(ep, stat)| async move {
            ep.get_container_by_id(&stat.id)
                .await?
                .ok_or_else(|| anyhow!("Failed to find existing container {}", stat.id))?
                .stop(stop_timeout)
                .await
                .map_err(Error::from)
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<()>>()
        .await
}


async fn images(endpoint_names: Vec<EndpointName>,
    matches: &ArgMatches,
    config: &Configuration,
) -> Result<()> {
    match matches.subcommand() {
        Some(("list", matches)) => images_list(endpoint_names, matches, config).await,
        Some(("verify-present", matches)) => images_present(endpoint_names, matches, config).await,
        Some((other, _)) => Err(anyhow!("Unknown subcommand: {}", other)),
        None => Err(anyhow!("No subcommand")),
    }
}

async fn images_list(endpoint_names: Vec<EndpointName>,
    _matches: &ArgMatches,
    config: &Configuration,
) -> Result<()> {
    let mut iter = connect_to_endpoints(config, &endpoint_names)
        .await?
        .into_iter()
        .map(move |ep| async move { ep.images(None).await })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<Vec<_>>>()
        .await?
        .into_iter()
        .flatten();

    let out = std::io::stdout();
    let mut lock = out.lock();

    iter.try_for_each(|img| {
        writeln!(lock, "{created} {id}", created = img.created(), id = {
            if let Some(tags)= img.tags() {
                tags.join(", ")
            } else {
                img.id().clone()
            }
        }).map_err(Error::from)
    })
}

async fn images_present(endpoint_names: Vec<EndpointName>,
    _matches: &ArgMatches,
    config: &Configuration,
) -> Result<()> {
    use crate::util::docker::ImageName;

    let eps = connect_to_endpoints(config, &endpoint_names).await?;

    let ep_names_to_images = eps.iter()
        .map(|ep| async move {
            ep.images(None).await.map(|imgs| {
                let img_tags = imgs.filter_map(|img| img.tags().clone().map(Vec::into_iter))
                    .flatten()
                    .map(ImageName::from)
                    .collect();

                (ep.name().clone(), img_tags)
            })
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<Vec<(EndpointName, Vec<ImageName>)>>>()
        .await?
        .into_iter()
        .collect::<HashMap<EndpointName, Vec<ImageName>>>();

    let out = std::io::stdout();
    let mut lock = out.lock();

    ep_names_to_images
        .iter()
        .try_for_each(|(ep_name, ep_imgs)| {
            config.docker()
                .images()
                .iter()
                .map(|config_img| (ep_imgs.contains(config_img), config_img))
                .try_for_each(|(found, img_name)| {
                    if found {
                        writeln!(lock, "found {img} in {ep}", img = img_name, ep = ep_name).map_err(Error::from)
                    } else {
                        writeln!(lock, "{img} not found", img = img_name).map_err(Error::from)
                    }
                })
        })
}

/// Helper function to connect to all endpoints from the configuration, that appear (by name) in
/// the `endpoint_names` list
pub(super) async fn connect_to_endpoints(config: &Configuration, endpoint_names: &[EndpointName]) -> Result<Vec<Arc<Endpoint>>> {
    let endpoint_configurations = config
        .docker()
        .endpoints()
        .iter()
        .filter(|(ep_name, _)| endpoint_names.contains(ep_name))
        .map(|(ep_name, ep_cfg)| {
            crate::endpoint::EndpointConfiguration::builder()
                .endpoint_name(ep_name.clone())
                .endpoint(ep_cfg.clone())
                .required_images(config.docker().images().clone())
                .required_docker_versions(config.docker().docker_versions().clone())
                .required_docker_api_versions(config.docker().docker_api_versions().clone())
                .build()
        })
        .collect::<Vec<_>>();

    info!("Endpoint config build");
    info!("Connecting to {n} endpoints: {eps}",
        n = endpoint_configurations.len(),
        eps = endpoint_configurations.iter().map(|epc| epc.endpoint_name()).join(", "));

    crate::endpoint::util::setup_endpoints(endpoint_configurations).await
}
