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
use shiplift::Container;

use crate::config::Configuration;

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

    let container = relevant_endpoint.get_container_by_id(container_id)
        .await?
        .ok_or_else(|| anyhow!("Cannot find container {} on {}", container_id, relevant_endpoint.name()))?;

    let confirm = |prompt: String| dialoguer::Confirm::new().with_prompt(prompt).interact();

    match matches.subcommand() {
        Some(("top", matches))  => top(matches, container).await,
        Some(("kill", matches)) => {
            confirm({
                if let Some(sig) = matches.value_of("signal").as_ref() {
                    format!("Really kill {} with {}?", container_id, sig)
                } else {
                    format!("Really kill {}?", container_id)
                }
            })?;

            kill(matches, container).await
        },
        Some(("delete", _)) => {
            confirm(format!("Really delete {}?", container_id))?;
            delete(container).await
        },
        Some(("start", _))      => {
            confirm(format!("Really start {}?", container_id))?;
            start(container).await
        },
        Some(("stop", matches)) => {
            confirm(format!("Really stop {}?", container_id))?;
            stop(matches, container).await
        },
        Some(("exec", matches)) => {
            confirm({
                let commands = matches.values_of("commands").unwrap().collect::<Vec<&str>>();
                format!("Really run '{}' in {}?", commands.join(" "), container_id)
            })?;
            exec(matches, container).await
        },
        Some((other, _)) => Err(anyhow!("Unknown subcommand: {}", other)),
        None => Err(anyhow!("No subcommand")),
    }
}

async fn top<'a>(matches: &ArgMatches, container: Container<'a>) -> Result<()> {
    let top = container.top(None).await?;
    let hdr = crate::commands::util::mk_header(top.titles.iter().map(|s| s.as_ref()).collect());
    crate::commands::util::display_data(hdr, top.processes, matches.is_present("csv"))
}

async fn kill<'a>(matches: &ArgMatches, container: Container<'a>) -> Result<()> {
    let signal = matches.value_of("signal");
    container.kill(signal).await.map_err(Error::from)
}

async fn delete<'a>(container: Container<'a>) -> Result<()> {
    container.delete().await.map_err(Error::from)
}

async fn start<'a>(container: Container<'a>) -> Result<()> {
    container.start().await.map_err(Error::from)
}

async fn stop<'a>(matches: &ArgMatches, container: Container<'a>) -> Result<()> {
    container.stop({
        matches
            .value_of("timeout")
            .map(u64::from_str)
            .transpose()?
            .map(std::time::Duration::from_secs)
    })
    .await
    .map_err(Error::from)
}

async fn exec<'a>(matches: &ArgMatches, container: Container<'a>) -> Result<()> {
    use std::io::Write;
    use futures::TryStreamExt;

    let execopts = shiplift::builder::ExecContainerOptions::builder()
        .cmd({
            matches.values_of("commands").unwrap().collect::<Vec<&str>>()
        })
        .attach_stdout(true)
        .attach_stderr(true)
        .build();

    container.exec(&execopts)
        .map_err(Error::from)
        .try_for_each(|chunk| async {
            match chunk {
                shiplift::tty::TtyChunk::StdIn(_) => Err(anyhow!("Cannot handle STDIN TTY chunk")),
                shiplift::tty::TtyChunk::StdOut(v) => {
                    std::io::stdout().write(&v).map_err(Error::from).map(|_| ())
                },
                shiplift::tty::TtyChunk::StdErr(v) => {
                    std::io::stderr().write(&v).map_err(Error::from).map(|_| ())
                },
            }
        })
        .await
}

