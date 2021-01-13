//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::anyhow;
use anyhow::Error;
use anyhow::Context;
use anyhow::Result;
use clap::ArgMatches;
use log::{info, trace};
use tokio::stream::StreamExt;
use tokio::io::AsyncWriteExt;

use crate::config::*;
use crate::package::Package;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;
use crate::source::*;
use crate::util::progress::ProgressBars;

pub async fn source(matches: &ArgMatches, config: &Configuration, repo: Repository, progressbars: ProgressBars) -> Result<()> {
    match matches.subcommand() {
        Some(("verify", matches))       => verify(matches, config, repo, progressbars).await,
        Some(("list-missing", matches)) => list_missing(matches, config, repo).await,
        Some(("url", matches))          => url(matches, repo).await,
        Some(("download", matches))     => download(matches, config, repo, progressbars).await,
        Some((other, _)) => return Err(anyhow!("Unknown subcommand: {}", other)),
        None             => return Err(anyhow!("No subcommand")),
    }
}

pub async fn verify(matches: &ArgMatches, config: &Configuration, repo: Repository, progressbars: ProgressBars) -> Result<()> {
    let sc                = SourceCache::new(config.source_cache_root().clone());
    let pname             = matches.value_of("package_name").map(String::from).map(PackageName::from);
    let pvers             = matches.value_of("package_version").map(String::from).map(PackageVersionConstraint::new).transpose()?;

    let packages = repo.packages()
        .filter(|p| pname.as_ref().map(|n| p.name() == n).unwrap_or(true))
        .filter(|p| pvers.as_ref().map(|v| v.matches(p.version())).unwrap_or(true));

    verify_impl(packages, &sc, &progressbars).await
}

pub (in crate::commands) async fn verify_impl<'a, I>(packages: I, sc: &SourceCache, progressbars: &ProgressBars) -> Result<()>
    where I: Iterator<Item = &'a Package> + 'a
{

    let multi = Arc::new(indicatif::MultiProgress::new());

    let results = packages
        .map(|p| sc.sources_for(p).into_iter())
        .flatten()
        .map(|src| (multi.clone(), src))
        .map(|(multi, source)| async move {
            trace!("Verifying: {}", source.path().display());
            let bar = multi.add(progressbars.bar());
            if source.path().exists() {
                trace!("Exists: {}", source.path().display());
                source.verify_hash()
                    .await
                    .with_context(|| anyhow!("Hash verification failed for: {}", source.path().display()))?;

                trace!("Success verifying: {}", source.path().display());
                let msg = format!("Ok: {}", source.path().display());
                bar.finish_with_message(&msg);
                Ok(msg)
            } else {
                trace!("Failed verifying: {}", source.path().display());
                bar.finish_with_message("Error");
                Err(anyhow!("Source missing: {}", source.path().display()))
            }
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Vec<Result<String>>>();

    let (results, _) = tokio::join!(results, async move { multi.join() });
    info!("Verification processes finished");

    let out = std::io::stdout();
    let mut any_error = false;
    for result in results {
        match result {
            Err(e) => {
                let mut outlock = out.lock();
                any_error = true;
                for cause in e.chain() {
                    let _ = writeln!(outlock, "{}", cause);
                }
                let _ = writeln!(outlock);
            },
            Ok(s) => {
                let _ = writeln!(out.lock(), "{}", s);
            },
        }
    }

    if any_error {
        Err(anyhow!("At least one package failed with source verification"))
    } else {
        Ok(())
    }
}

pub async fn list_missing(_: &ArgMatches, config: &Configuration, repo: Repository) -> Result<()> {
    let sc          = SourceCache::new(config.source_cache_root().clone());
    let out         = std::io::stdout();
    let mut outlock = out.lock();

    repo.packages()
        .try_for_each(|p| {
            for source in sc.sources_for(p) {
                if !source.exists() {
                    writeln!(outlock, "{} {} -> {}", p.name(), p.version(), source.path().display())?;
                }
            }

            Ok(())
        })
}

pub async fn url(matches: &ArgMatches, repo: Repository) -> Result<()> {
    let out         = std::io::stdout();
    let mut outlock = out.lock();

    let pname = matches.value_of("package_name").map(String::from).map(PackageName::from);
    let pvers = matches.value_of("package_version").map(String::from).map(PackageVersionConstraint::new).transpose()?;

    repo.packages()
        .filter(|p| pname.as_ref().map(|n| p.name() == n).unwrap_or(true))
        .filter(|p| pvers.as_ref().map(|v| v.matches(p.version())).unwrap_or(true))
        .try_for_each(|p| {
            p.sources()
                .iter()
                .try_for_each(|(source_name, source)| {
                    writeln!(outlock, "{} {} -> {} = {}", p.name(), p.version(), source_name, source.url())
                        .map_err(Error::from)
                })
        })
}

pub async fn download(matches: &ArgMatches, config: &Configuration, repo: Repository, progressbars: ProgressBars) -> Result<()> {
    let force = matches.is_present("force");
    let cache = PathBuf::from(config.source_cache_root());
    let sc    = SourceCache::new(cache);
    let pname = matches.value_of("package_name").map(String::from).map(PackageName::from);
    let pvers = matches.value_of("package_version").map(String::from).map(PackageVersionConstraint::new).transpose()?;
    let multi = indicatif::MultiProgress::new();

    let r = repo.packages()
        .filter(|p| pname.as_ref().map(|n| p.name() == n).unwrap_or(true))
        .filter(|p| pvers.as_ref().map(|v| v.matches(p.version())).unwrap_or(true))
        .map(|p| {
            sc.sources_for(p)
                .into_iter()
                .map(|source| {
                    let bar = multi.add(progressbars.spinner());
                    bar.set_message(&format!("Downloading {}", source.url()));
                    async move {
                        if !source.exists() && source.download_manually() {
                            return Err(anyhow!("Cannot download source that is marked for manual download"))
                                .context(anyhow!("Creating source: {}", source.path().display()))
                                .context(anyhow!("Downloading source: {}", source.url()))
                                .map_err(Error::from)
                        }

                        if source.exists() && !force {
                            Err(anyhow!("Source exists: {}", source.path().display()))
                        } else {
                            if source.exists() {
                                let _ = source.remove_file().await?;
                            }

                            trace!("Creating: {:?}", source);
                            let file = source.create().await
                                .with_context(|| anyhow!("Creating source file destination: {}", source.path().display()))?;

                            let mut file = tokio::io::BufWriter::new(file);
                            let response = reqwest::get(source.url().as_ref()).await
                                .with_context(|| anyhow!("Downloading '{}'", source.url()))?;

                            if let Some(len) = response.content_length() {
                                bar.set_length(len);
                            }
                            let mut stream = reqwest::get(source.url().as_ref()).await?.bytes_stream();
                            while let Some(bytes) = stream.next().await {
                                let bytes = bytes?;
                                file.write_all(bytes.as_ref()).await?;
                                bar.inc(bytes.len() as u64);
                            }

                            file.flush().await?;
                            bar.finish_with_message("Finished download");
                            Ok(())
                        }
                    }
                })
        })
        .flatten()
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<()>>()
        .await;
    multi.join()?;
    r
}

