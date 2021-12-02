//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'find-artifact' subcommand

use std::path::PathBuf;
use std::io::Write;
use std::sync::Arc;
use std::convert::TryFrom;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use diesel::PgConnection;
use itertools::Itertools;
use log::debug;
use log::trace;

use crate::config::Configuration;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::filestore::path::StoreRoot;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;
use crate::util::progress::ProgressBars;
use crate::util::docker::ImageName;

/// Implementation of the "find_artifact" subcommand
pub async fn find_artifact(matches: &ArgMatches, config: &Configuration, progressbars: ProgressBars, repo: Repository, database_connection: PgConnection) -> Result<()> {
    let package_name_regex = crate::commands::util::mk_package_name_regex({
        matches.value_of("package_name_regex").unwrap() // safe by clap
    })?;

    let package_version_constraint = matches
        .value_of("package_version_constraint")
        .map(PackageVersionConstraint::try_from)
        .transpose()
        .context("Parsing package version constraint")
        .context("A valid package version constraint looks like this: '=1.0.0'")?;

    let env_filter = matches.values_of("env_filter")
        .map(|vals| vals.map(crate::util::env::parse_to_env).collect::<Result<Vec<_>>>())
        .transpose()?
        .unwrap_or_default();

    let image_name = matches.value_of("image")
        .map(String::from)
        .map(ImageName::from);

    log::debug!("Finding artifacts for '{:?}' '{:?}'", package_name_regex, package_version_constraint);

    let release_stores = config
        .release_stores()
        .iter()
        .map(|storename| {
            let bar_release_loading = progressbars.bar();

            let p = config.releases_directory().join(storename);
            debug!("Loading release directory: {}", p.display());
            let r = ReleaseStore::load(StoreRoot::new(p)?, &bar_release_loading);
            if r.is_ok() {
                bar_release_loading.finish_with_message("Loaded releases successfully");
            } else {
                bar_release_loading.finish_with_message("Failed to load releases");
            }

            r.map(Arc::new)
        })
        .collect::<Result<Vec<_>>>()?;

    let staging_store = if let Some(p) = matches.value_of("staging_dir").map(PathBuf::from) {
        let bar_staging_loading = progressbars.bar();

        if !p.is_dir() {
            let _ = tokio::fs::create_dir_all(&p).await?;
        }

        debug!("Loading staging directory: {}", p.display());
        let r = StagingStore::load(StoreRoot::new(p.clone())?, &bar_staging_loading);
        if r.is_ok() {
            bar_staging_loading.finish_with_message("Loaded staging successfully");
        } else {
            bar_staging_loading.finish_with_message("Failed to load staging");
        }
        Some(r?)
    } else {
        None
    };

    let database = Arc::new(database_connection);
    repo.packages()
        .filter(|p| package_name_regex.captures(p.name()).is_some())
        .filter(|p| {
            package_version_constraint
                .as_ref()
                .map(|v| v.matches(p.version()))
                .unwrap_or(true)
        })
        .inspect(|pkg| trace!("Found package: {:?}", pkg))
        .map(|pkg| {
            let script_filter = !matches.is_present("no_script_filter");
            let pathes = crate::db::FindArtifacts::builder()
                .config(config)
                .release_stores(&release_stores)
                .staging_store(staging_store.as_ref())
                .database_connection(database.clone())
                .env_filter(&env_filter)
                .script_filter(script_filter)
                .image_name(image_name.as_ref())
                .package(pkg)
                .build()
                .run()?;

            pathes.iter()
                .map(|tpl| (tpl.0.joined(), tpl.1))
                .sorted_by(|tpla, tplb| {
                    use std::cmp::Ordering;

                    // Sort the iterator elements, so that if there is a release date, we always
                    // prefer the entry with the release date AS LONG AS the path is equal.
                    match (tpla, tplb) {
                        ((a, Some(ta)), (b, Some(tb))) => match a.cmp(b) {
                            Ordering::Equal => ta.cmp(tb),
                            other => other,
                        },

                        ((a, Some(_)), (b, None)) => match a.cmp(b) {
                            Ordering::Equal => Ordering::Greater,
                            other => other,
                        },
                        ((a, None), (b, Some(_))) => match a.cmp(b) {
                            Ordering::Equal => Ordering::Less,
                            other => other,
                        },
                        ((a, None), (b, None)) => a.cmp(b),
                    }
                })
                .unique_by(|tpl| tpl.0.clone()) // TODO: Dont clone()
                .try_for_each(|(path, releasetime)| {
                    if let Some(time) = releasetime {
                        writeln!(std::io::stdout(), "[{}] {}", time, path.display())
                    } else {
                        writeln!(std::io::stdout(), "[unknown] {}", path.display())
                    }.map_err(Error::from)
                })
        })
        .inspect(|r| trace!("Query resulted in: {:?}", r))
        .collect::<Vec<Result<()>>>()
        .into_iter()
        .collect()
}
