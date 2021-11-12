//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'source' subcommand

use std::io::Write;
use std::path::PathBuf;
use std::convert::TryFrom;
use std::str::FromStr;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use colored::Colorize;
use log::{info, trace};
use tokio::io::AsyncWriteExt;
use tokio_stream::StreamExt;

use crate::config::*;
use crate::package::Package;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;
use crate::source::*;
use crate::util::progress::ProgressBars;

/// Implementation of the "source" subcommand
pub async fn source(
    matches: &ArgMatches,
    config: &Configuration,
    repo: Repository,
    progressbars: ProgressBars,
) -> Result<()> {
    match matches.subcommand() {
        Some(("verify", matches)) => verify(matches, config, repo, progressbars).await,
        Some(("list-missing", matches)) => list_missing(matches, config, repo).await,
        Some(("url", matches)) => url(matches, repo).await,
        Some(("download", matches)) => download(matches, config, repo, progressbars).await,
        Some(("of", matches)) => of(matches, config, repo).await,
        Some((other, _)) => return Err(anyhow!("Unknown subcommand: {}", other)),
        None => Err(anyhow!("No subcommand")),
    }
}

pub async fn verify(
    matches: &ArgMatches,
    config: &Configuration,
    repo: Repository,
    progressbars: ProgressBars,
) -> Result<()> {
    let sc = SourceCache::new(config.source_cache_root().clone());
    let pname = matches
        .value_of("package_name")
        .map(String::from)
        .map(PackageName::from);
    let pvers = matches
        .value_of("package_version")
        .map(PackageVersionConstraint::try_from)
        .transpose()?;

    let matching_regexp = matches.value_of("matching")
        .map(crate::commands::util::mk_package_name_regex)
        .transpose()?;

    let packages = repo
        .packages()
        .filter(|p| {
            match (pname.as_ref(), pvers.as_ref(), matching_regexp.as_ref()) {
                (None, None, None)              => true,
                (Some(pname), None, None)       => p.name() == pname,
                (Some(pname), Some(vers), None) => p.name() == pname && vers.matches(p.version()),
                (None, None, Some(regex))       => regex.is_match(p.name()),

                (_, _, _) => {
                    panic!("This should not be possible, either we select packages by name and (optionally) version, or by regex.")
                },
            }
        })
        .inspect(|p| trace!("Found for verification: {} {}", p.name(), p.version()));

    verify_impl(packages, &sc, &progressbars).await
}

pub(in crate::commands) async fn verify_impl<'a, I>(
    packages: I,
    sc: &SourceCache,
    progressbars: &ProgressBars,
) -> Result<()>
where
    I: Iterator<Item = &'a Package> + 'a,
{
    let sources = packages
        .map(|p| sc.sources_for(p).into_iter())
        .flatten()
        .collect::<Vec<_>>();

    let bar = progressbars.bar();
    bar.set_message("Verifying sources");
    bar.set_length(sources.len() as u64);

    let results = sources.into_iter()
        .map(|src| (bar.clone(), src))
        .map(|(bar, source)| async move {
            trace!("Verifying: {}", source.path().display());
            if source.path().exists() {
                trace!("Exists: {}", source.path().display());
                source.verify_hash().await.with_context(|| {
                    anyhow!("Hash verification failed for: {}", source.path().display())
                })?;

                trace!("Success verifying: {}", source.path().display());
                bar.inc(1);
                Ok(())
            } else {
                trace!("Failed verifying: {}", source.path().display());
                bar.inc(1);
                Err(anyhow!("Source missing: {}", source.path().display()))
            }
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Vec<Result<_>>>()
        .await;

    info!("Verification processes finished");

    if results.iter().any(Result::is_err) {
        bar.finish_with_message("Source verification failed");
    } else {
        bar.finish_with_message("Source verification successfull");
    }

    let out = std::io::stdout();
    let mut any_error = false;
    for result in results {
        if let Err(e) = result {
            let mut outlock = out.lock();
            any_error = true;
            for cause in e.chain() {
                let _ = writeln!(outlock, "Error: {}", cause.to_string().red());
            }
            let _ = writeln!(outlock);
        }
    }

    if any_error {
        Err(anyhow!(
            "At least one package failed with source verification"
        ))
    } else {
        Ok(())
    }
}

pub async fn list_missing(_: &ArgMatches, config: &Configuration, repo: Repository) -> Result<()> {
    let sc = SourceCache::new(config.source_cache_root().clone());
    let out = std::io::stdout();
    let mut outlock = out.lock();

    repo.packages().try_for_each(|p| {
        for source in sc.sources_for(p) {
            if !source.path().exists() {
                writeln!(
                    outlock,
                    "{} {} -> {}",
                    p.name(),
                    p.version(),
                    source.path().display()
                )?;
            }
        }

        Ok(())
    })
}

pub async fn url(matches: &ArgMatches, repo: Repository) -> Result<()> {
    let out = std::io::stdout();
    let mut outlock = out.lock();

    let pname = matches
        .value_of("package_name")
        .map(String::from)
        .map(PackageName::from);
    let pvers = matches
        .value_of("package_version")
        .map(PackageVersionConstraint::try_from)
        .transpose()?;

    repo.packages()
        .filter(|p| pname.as_ref().map(|n| p.name() == n).unwrap_or(true))
        .filter(|p| {
            pvers
                .as_ref()
                .map(|v| v.matches(p.version()))
                .unwrap_or(true)
        })
        .try_for_each(|p| {
            p.sources().iter().try_for_each(|(source_name, source)| {
                writeln!(
                    outlock,
                    "{} {} -> {} = {}",
                    p.name(),
                    p.version(),
                    source_name,
                    source.url()
                )
                .map_err(Error::from)
            })
        })
}

pub async fn download(
    matches: &ArgMatches,
    config: &Configuration,
    repo: Repository,
    progressbars: ProgressBars,
) -> Result<()> {
    async fn perform_download(source: &SourceEntry, bar: &indicatif::ProgressBar, timeout: Option<u64>) -> Result<()> {
        trace!("Creating: {:?}", source);
        let file = source.create().await.with_context(|| {
            anyhow!(
                "Creating source file destination: {}",
                source.path().display()
            )
        })?;

        let mut file = tokio::io::BufWriter::new(file);
        let client_builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10));

        let client_builder = if let Some(to) = timeout {
            client_builder.timeout(std::time::Duration::from_secs(to))
        } else {
            client_builder
        };

        let client = client_builder.build().context("Building HTTP client failed")?;

        let request = client.get(source.url().as_ref())
            .build()
            .with_context(|| anyhow!("Building request for {} failed", source.url().as_ref()))?;

        let response = match client.execute(request).await {
            Ok(resp) => resp,
            Err(e) => {
                bar.finish_with_message(format!("Failed: {}", source.url()));
                return Err(e).with_context(|| anyhow!("Downloading '{}'", source.url()))
            }
        };

        if let Some(len) = response.content_length() {
            bar.set_length(len);
        }

        let mut stream = reqwest::get(source.url().as_ref()).await?.bytes_stream();
        let mut bytes_written = 0;
        while let Some(bytes) = stream.next().await {
            let bytes = bytes?;
            file.write_all(bytes.as_ref()).await?;
            bytes_written += bytes.len();

            bar.inc(bytes.len() as u64);
            if let Some(len) = response.content_length() {
                bar.set_message(format!("Downloading {} ({}/{} bytes)", source.url(), bytes_written, len));
            } else {
                bar.set_message(format!("Downloading {} ({} bytes)", source.url(), bytes_written));
            }
        }

        file.flush()
            .await
            .map_err(Error::from)
            .map(|_| ())
    }

    let force = matches.is_present("force");
    let timeout = matches.value_of("timeout")
        .map(u64::from_str)
        .transpose()
        .context("Parsing timeout argument to integer")?;
    let cache = PathBuf::from(config.source_cache_root());
    let sc = SourceCache::new(cache);
    let pname = matches
        .value_of("package_name")
        .map(String::from)
        .map(PackageName::from);
    let pvers = matches
        .value_of("package_version")
        .map(PackageVersionConstraint::try_from)
        .transpose()?;
    let multi = {
        let mp = indicatif::MultiProgress::new();
        if progressbars.hide() {
            mp.set_draw_target(indicatif::ProgressDrawTarget::hidden());
        }
        mp
    };

    let matching_regexp = matches.value_of("matching")
        .map(crate::commands::util::mk_package_name_regex)
        .transpose()?;

    let r = repo
        .packages()
        .filter(|p| {
            match (pname.as_ref(), pvers.as_ref(), matching_regexp.as_ref()) {
                (None, None, None)              => true,
                (Some(pname), None, None)       => p.name() == pname,
                (Some(pname), Some(vers), None) => p.name() == pname && vers.matches(p.version()),
                (None, None, Some(regex))       => regex.is_match(p.name()),

                (_, _, _) => {
                    panic!("This should not be possible, either we select packages by name and (optionally) version, or by regex.")
                },
            }
        })
        .map(|p| {
            sc.sources_for(p).into_iter().map(|source| {
                let bar = multi.add(progressbars.spinner());
                bar.set_message(format!("Downloading {}", source.url()));
                async move {
                    let source_path_exists = source.path().exists();
                    if !source_path_exists && source.download_manually() {
                        return Err(anyhow!(
                            "Cannot download source that is marked for manual download"
                        ))
                        .context(anyhow!("Creating source: {}", source.path().display()))
                        .context(anyhow!("Downloading source: {}", source.url()))
                        .map_err(Error::from);
                    }

                    if source_path_exists && !force {
                        Err(anyhow!("Source exists: {}", source.path().display()))
                    } else {
                        if source_path_exists /* && force is implied by 'if' above*/ {
                            if let Err(e) = source.remove_file().await {
                                bar.finish_with_message(format!("Failed to remove existing file: {}", source.path().display()));
                                return Err(e)
                            }
                        }


                        if let Err(e) = perform_download(&source, &bar, timeout).await {
                            bar.finish_with_message(format!("Failed: {}", source.url()));
                            Err(e)
                        } else {
                            bar.finish_with_message(format!("Finished: {}", source.url()));
                            Ok(())
                        }
                    }
                }
            })
        })
        .flatten()
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Vec<Result<()>>>();

    let multibar_block = tokio::task::spawn_blocking(move || multi.join());
    let (r, _) = tokio::join!(r, multibar_block);
    r.into_iter().collect()
}

async fn of(
    matches: &ArgMatches,
    config: &Configuration,
    repo: Repository,
) -> Result<()> {
    let cache = PathBuf::from(config.source_cache_root());
    let sc = SourceCache::new(cache);
    let pname = matches
        .value_of("package_name")
        .map(String::from)
        .map(PackageName::from);
    let pvers = matches
        .value_of("package_version")
        .map(PackageVersionConstraint::try_from)
        .transpose()?;

    repo.packages()
        .filter(|p| pname.as_ref().map(|n| p.name() == n).unwrap_or(true))
        .filter(|p| {
            pvers
                .as_ref()
                .map(|v| v.matches(p.version()))
                .unwrap_or(true)
        })
        .map(|p| {
            let pathes = sc.sources_for(p)
                .into_iter()
                .map(|source| source.path())
                .collect::<Vec<PathBuf>>();

            (p, pathes)
        })
        .fold(Ok(std::io::stdout()), |out, (package, pathes)| {
            out.and_then(|mut out| {
                writeln!(out, "{} {}", package.name(), package.version())?;
                for path in pathes {
                    writeln!(out, "\t{}", path.display())?;
                }

                Ok(out)
            })
        })
        .map(|_| ())
}
