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

use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use clap::ArgMatches;
use tokio_stream::StreamExt;

use crate::config::Configuration;
use crate::endpoint::Endpoint;

pub async fn container(endpoint_names: Vec<String>,
    matches: &ArgMatches,
    config: &Configuration,
) -> Result<()> {
    let container_id = matches.value_of("container_id").unwrap();
    let endpoints = crate::commands::endpoint::connect_to_endpoints(config, &endpoint_names).await?;
    let relevant_endpoints = endpoints.into_iter()
        .map(|ep| async {
            ep.has_container_with_id(container_id)
                .await
                .map(|b| (ep, b))
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<Vec<(_, bool)>>>()
        .await?
        .into_iter()
        .filter_map(|tpl| {
            if tpl.1 {
                Some(tpl.0)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    if relevant_endpoints.len() > 1 {
        return Err(anyhow!("Found more than one container for id {}", container_id))
    }

    let relevant_endpoint = relevant_endpoints.get(0).ok_or_else(|| {
        anyhow!("Found no container for id {}", container_id)
    })?;

    match matches.subcommand() {
        Some(("top", matches)) => container_top(matches, relevant_endpoint, container_id).await,
        Some(("kill", matches)) => container_kill(matches, relevant_endpoint, container_id).await,
        Some(("delete", _)) => container_delete(relevant_endpoint, container_id).await,
        Some(("start", _)) => container_start(relevant_endpoint, container_id).await,
        Some(("stop", matches)) => container_stop(matches, relevant_endpoint, container_id).await,
        Some(("exec", matches)) => container_exec(matches, relevant_endpoint, container_id).await,
        Some((other, _)) => Err(anyhow!("Unknown subcommand: {}", other)),
        None => Err(anyhow!("No subcommand")),
    }
}

async fn container_top(
    matches: &ArgMatches,
    endpoint: &Endpoint,
    container_id: &str,
) -> Result<()> {
    let csv = matches.is_present("csv");
    let top = endpoint
        .get_container_by_id(container_id)
        .await?
        .ok_or_else(|| anyhow!("Cannot find container {} on {}", container_id, endpoint.name()))?
        .top(None)
        .await?;

    let hdr = crate::commands::util::mk_header(top.titles.iter().map(|s| s.as_ref()).collect());
    crate::commands::util::display_data(hdr, top.processes, csv)
}

async fn container_kill(
    matches: &ArgMatches,
    endpoint: &Endpoint,
    container_id: &str,
) -> Result<()> {
    let signal = matches.value_of("signal");
    let prompt = if let Some(sig) = signal.as_ref() {
        format!("Really kill {} with {}?", container_id, sig)
    } else {
        format!("Really kill {}?", container_id)
    };

    dialoguer::Confirm::new().with_prompt(prompt).interact()?;

    endpoint
        .get_container_by_id(container_id)
        .await?
        .ok_or_else(|| anyhow!("Cannot find container {} on {}", container_id, endpoint.name()))?
        .kill(signal)
        .await
        .map_err(Error::from)
}

async fn container_delete(
    endpoint: &Endpoint,
    container_id: &str,
) -> Result<()> {
    let prompt = format!("Really delete {}?", container_id);
    dialoguer::Confirm::new().with_prompt(prompt).interact()?;

    endpoint
        .get_container_by_id(container_id)
        .await?
        .ok_or_else(|| anyhow!("Cannot find container {} on {}", container_id, endpoint.name()))?
        .delete()
        .await
        .map_err(Error::from)
}

async fn container_start(
    endpoint: &Endpoint,
    container_id: &str,
) -> Result<()> {
    let prompt = format!("Really start {}?", container_id);
    dialoguer::Confirm::new().with_prompt(prompt).interact()?;

    endpoint
        .get_container_by_id(container_id)
        .await?
        .ok_or_else(|| anyhow!("Cannot find container {} on {}", container_id, endpoint.name()))?
        .start()
        .await
        .map_err(Error::from)
}

async fn container_stop(
    matches: &ArgMatches,
    endpoint: &Endpoint,
    container_id: &str,
) -> Result<()> {
    let timeout = matches.value_of("timeout").map(u64::from_str).transpose()?.map(std::time::Duration::from_secs);
    let prompt = format!("Really stop {}?", container_id);
    dialoguer::Confirm::new().with_prompt(prompt).interact()?;

    endpoint
        .get_container_by_id(container_id)
        .await?
        .ok_or_else(|| anyhow!("Cannot find container {} on {}", container_id, endpoint.name()))?
        .stop(timeout)
        .await
        .map_err(Error::from)
}

async fn container_exec(
    matches: &ArgMatches,
    endpoint: &Endpoint,
    container_id: &str,
) -> Result<()> {
    use std::io::Write;
    use futures::TryStreamExt;

    let commands = matches.values_of("commands").unwrap().collect::<Vec<&str>>();
    let prompt = format!("Really run '{}' in {}?", commands.join(" "), container_id);
    dialoguer::Confirm::new().with_prompt(prompt).interact()?;

    let execopts = shiplift::builder::ExecContainerOptions::builder()
        .cmd(commands)
        .attach_stdout(true)
        .attach_stderr(true)
        .build();

    endpoint
        .get_container_by_id(container_id)
        .await?
        .ok_or_else(|| anyhow!("Cannot find container {} on {}", container_id, endpoint.name()))?
        .exec(&execopts)
        .map_err(Error::from)
        .try_for_each(|chunk| async {
            let mut stdout = std::io::stdout();
            let mut stderr = std::io::stderr();
            match chunk {
                shiplift::tty::TtyChunk::StdIn(_) => Err(anyhow!("Cannot handle STDIN TTY chunk")),
                shiplift::tty::TtyChunk::StdOut(v) => stdout.write(&v).map_err(Error::from).map(|_| ()),
                shiplift::tty::TtyChunk::StdErr(v) => stderr.write(&v).map_err(Error::from).map(|_| ()),
            }
        })
        .await
}

