//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'release' subcommand

use std::io::Write;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use diesel::prelude::*;
use log::{debug, info, trace};
use tokio_stream::StreamExt;

use crate::config::Configuration;
use crate::db::models as dbmodels;
use crate::db::DbConnectionConfig;

/// Implementation of the "release" subcommand
pub async fn release(
    db_connection_config: DbConnectionConfig<'_>,
    config: &Configuration,
    matches: &ArgMatches,
) -> Result<()> {
    match matches.subcommand() {
        Some(("new", matches))  => new_release(db_connection_config, config, matches).await,
        Some(("rm", matches))   => rm_release(db_connection_config, config, matches).await,
        Some((other, _matches)) => Err(anyhow!("Unknown subcommand: {}", other)),
        None => Err(anyhow!("Missing subcommand")),
    }
}


async fn new_release(
    db_connection_config: DbConnectionConfig<'_>,
    config: &Configuration,
    matches: &ArgMatches,
) -> Result<()> {
    let print_released_file_pathes = !matches.is_present("quiet");
    let release_store_name = matches.value_of("release_store_name").unwrap(); // safe by clap
    if !(config.releases_directory().exists() && config.releases_directory().is_dir()) {
        return Err(anyhow!(
            "Release directory does not exist or does not point to directory: {}",
            config.releases_directory().display()
        ));
    }

    let pname = matches.value_of("package_name").map(String::from);

    let pvers = matches.value_of("package_version").map(String::from);

    debug!("Release called for: {:?} {:?}", pname, pvers);

    let conn = db_connection_config.establish_connection()?;
    let submit_uuid = matches
        .value_of("submit_uuid")
        .map(uuid::Uuid::parse_str)
        .transpose()?
        .unwrap(); // safe by clap
    debug!("Release called for submit: {:?}", submit_uuid);

    let submit = crate::schema::submits::dsl::submits
        .filter(crate::schema::submits::dsl::uuid.eq(submit_uuid))
        .first::<dbmodels::Submit>(&conn)?;
    debug!("Found Submit: {:?}", submit_uuid);

    let arts = {
        let sel = crate::schema::artifacts::dsl::artifacts
            .inner_join(crate::schema::jobs::table.inner_join(crate::schema::packages::table))
            .filter(crate::schema::jobs::submit_id.eq(submit.id))
            .left_outer_join(crate::schema::releases::table) // not released
            .select(crate::schema::artifacts::all_columns);

        match (pname, pvers) {
            (Some(name), Some(vers)) => {
                let query = sel
                    .filter(crate::schema::packages::name.eq(name))
                    .filter(crate::schema::packages::version.like(vers));
                debug!(
                    "Query: {:?}",
                    diesel::debug_query::<diesel::pg::Pg, _>(&query)
                );
                query.load::<dbmodels::Artifact>(&conn)?
            }
            (Some(name), None) => {
                let query = sel.filter(crate::schema::packages::name.eq(name));
                debug!(
                    "Query: {:?}",
                    diesel::debug_query::<diesel::pg::Pg, _>(&query)
                );
                query.load::<dbmodels::Artifact>(&conn)?
            }
            (None, Some(vers)) => {
                let query = sel.filter(crate::schema::packages::version.like(vers));
                debug!(
                    "Query: {:?}",
                    diesel::debug_query::<diesel::pg::Pg, _>(&query)
                );
                query.load::<dbmodels::Artifact>(&conn)?
            }
            (None, None) => {
                debug!(
                    "Query: {:?}",
                    diesel::debug_query::<diesel::pg::Pg, _>(&sel)
                );
                sel.load::<dbmodels::Artifact>(&conn)?
            }
        }
    };
    debug!("Artifacts = {:?}", arts);

    arts.iter()
        .filter_map(|art| {
            art.path_buf()
                .parent()
                .map(|p| config.releases_directory().join(release_store_name).join(p))
        })
        .map(|p| async {
            debug!("mkdir {:?}", p);
            tokio::fs::create_dir_all(p).await.map_err(Error::from)
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<()>>()
        .await?;

    let staging_base: &PathBuf = &config.staging_directory().join(submit.uuid.to_string());

    let release_store = crate::db::models::ReleaseStore::create(&conn, release_store_name)?;
    let do_update = matches.is_present("package_do_update");
    let interactive = !matches.is_present("noninteractive");

    let now = chrono::offset::Local::now().naive_local();
    arts.into_iter()
        .map(|art| async move {
            let art_path = staging_base.join(&art.path);
            let dest_path = config.releases_directory().join(release_store_name).join(&art.path);
            debug!(
                "Trying to release {} to {}",
                art_path.display(),
                dest_path.display()
            );

            if !art_path.is_file() {
                trace!(
                    "Artifact does not exist as file, cannot release it: {:?}",
                    art
                );
                Err(anyhow!("Not a file: {}", art_path.display()))
            } else {
                if dest_path.exists() && !do_update {
                    return Err(anyhow!("Does already exist: {}", dest_path.display()));
                } else if dest_path.exists() && do_update {
                    writeln!(std::io::stderr(), "Going to update: {}", dest_path.display())?;
                    if interactive && !dialoguer::Confirm::new().with_prompt("Continue?").interact()? {
                        return Err(anyhow!("Does already exist: {} and update was denied", dest_path.display()));
                    }
                }

                // else !dest_path.exists()
                tokio::fs::copy(art_path, &dest_path)
                    .await
                    .map_err(Error::from)
                    .map(|_| (art, dest_path))
            }
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<Vec<_>>>()
        .await?
        .into_iter()
        .try_for_each(|(art, dest_path)| {
            debug!("Updating {:?} to set released = true", art);
            let rel = crate::db::models::Release::create(&conn, &art, &now, &release_store)?;
            debug!("Release object = {:?}", rel);

            if print_released_file_pathes {
                writeln!(std::io::stdout(), "{}", dest_path.display()).map_err(Error::from)
            } else {
                Ok(())
            }
        })
}

pub async fn rm_release(
    db_connection_config: DbConnectionConfig<'_>,
    config: &Configuration,
    matches: &ArgMatches,
) -> Result<()> {
    let release_store_name = matches.value_of("release_store_name").map(String::from).unwrap(); // safe by clap
    if !(config.releases_directory().exists() && config.releases_directory().is_dir()) {
        return Err(anyhow!(
            "Release directory does not exist or does not point to directory: {}",
            config.releases_directory().display()
        ));
    }
    if !config.release_stores().contains(&release_store_name) {
        return Err(anyhow!("Unknown release store name: {}", release_store_name))
    }

    let pname = matches.value_of("package_name").map(String::from).unwrap(); // safe by clap
    let pvers = matches.value_of("package_version").map(String::from).unwrap(); // safe by clap
    debug!("Remove Release called for: {:?} {:?}", pname, pvers);

    let conn = db_connection_config.establish_connection()?;

    let (release, artifact) = crate::schema::jobs::table
        .inner_join(crate::schema::packages::table)
        .inner_join(crate::schema::artifacts::table)
        .inner_join(crate::schema::releases::table
            .on(crate::schema::releases::artifact_id.eq(crate::schema::artifacts::id)))
        .inner_join(crate::schema::release_stores::table
            .on(crate::schema::release_stores::id.eq(crate::schema::releases::release_store_id)))
        .filter(crate::schema::packages::dsl::name.eq(&pname)
            .and(crate::schema::packages::dsl::version.eq(&pvers)))
        .filter(crate::schema::release_stores::dsl::store_name.eq(&release_store_name))
        .order(crate::schema::releases::dsl::release_date.desc())
        .select((crate::schema::releases::all_columns, crate::schema::artifacts::all_columns))
        .first::<(crate::db::models::Release, crate::db::models::Artifact)>(&conn)?;

    let artifact_path = config.releases_directory().join(release_store_name).join(&artifact.path);
    if !artifact_path.is_file() {
        return Err(anyhow!("Not a file: {}", artifact_path.display()))
    }

    writeln!(std::io::stderr(), "Going to delete: {}", artifact_path.display())?;
    writeln!(std::io::stderr(), "Going to remove from database: Release with ID {} from {}", release.id, release.release_date)?;
    if !dialoguer::Confirm::new().with_prompt("Continue?").interact()? {
        return Ok(())
    }

    tokio::fs::remove_file(&artifact_path).await?;
    info!("File removed");

    diesel::delete(&release).execute(&conn)?;
    info!("Release deleted from database");

    Ok(())
}

