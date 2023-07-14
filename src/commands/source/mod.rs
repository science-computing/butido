//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'source' subcommand

use std::convert::TryFrom;
use std::io::Write;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use colored::Colorize;
use tokio_stream::StreamExt;
use tracing::{info, trace};

use crate::config::*;
use crate::package::Package;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;
use crate::source::*;
use crate::util::progress::ProgressBars;

mod download;

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
        Some(("download", matches)) => {
            crate::commands::source::download::download(matches, config, repo, progressbars).await
        }
        Some(("of", matches)) => of(matches, config, repo).await,
        Some((other, _)) => Err(anyhow!("Unknown subcommand: {}", other)),
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
        .get_one::<String>("package_name")
        .map(|s| s.to_owned())
        .map(PackageName::from);
    let pvers = matches
        .get_one::<String>("package_version")
        .map(|s| s.to_owned())
        .map(PackageVersionConstraint::try_from)
        .transpose()?;

    let matching_regexp = matches
        .get_one::<String>("matching")
        .map(|s| crate::commands::util::mk_package_name_regex(s.as_ref()))
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
        .flat_map(|p| sc.sources_for(p).into_iter())
        .collect::<Vec<_>>();

    let bar = progressbars.bar()?;
    bar.set_message("Verifying sources");
    bar.set_length(sources.len() as u64);

    let results = sources
        .into_iter()
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
        bar.finish_with_message("Source verification successful");
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
        .get_one::<String>("package_name")
        .map(|s| s.to_owned())
        .map(PackageName::from);
    let pvers = matches
        .get_one::<String>("package_version")
        .map(|s| s.to_owned())
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

async fn of(matches: &ArgMatches, config: &Configuration, repo: Repository) -> Result<()> {
    let cache = PathBuf::from(config.source_cache_root());
    let sc = SourceCache::new(cache);
    let pname = matches
        .get_one::<String>("package_name")
        .map(|s| s.to_owned())
        .map(PackageName::from);
    let pvers = matches
        .get_one::<String>("package_version")
        .map(|s| s.to_owned())
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
            let pathes = sc
                .sources_for(p)
                .into_iter()
                .map(|source| source.path())
                .collect::<Vec<PathBuf>>();

            (p, pathes)
        })
        .try_fold(std::io::stdout(), |mut out, (package, pathes)| {
            writeln!(out, "{} {}", package.name(), package.version())?;
            for path in pathes {
                writeln!(out, "\t{}", path.display())?;
            }

            Ok(out)
        })
        .map(|_| ())
}
