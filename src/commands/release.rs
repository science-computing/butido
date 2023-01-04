//
// Copyright (c) 2020-2022 science+computing ag and other contributors
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
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use diesel::prelude::*;
use tracing::{debug, error, info, trace};
use tokio_stream::StreamExt;
use resiter::AndThen;

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
    let print_released_file_pathes = !matches.get_flag("quiet");
    let release_store_name = matches.get_one::<String>("release_store_name").unwrap(); // safe by clap
    if !(config.releases_directory().exists() && config.releases_directory().is_dir()) {
        return Err(anyhow!(
            "Release directory does not exist or does not point to directory: {}",
            config.releases_directory().display()
        ));
    }

    let pname = matches.get_one::<String>("package_name");

    let pvers = matches.get_one::<String>("package_version");

    debug!("Release called for: {:?} {:?}", pname, pvers);

    let mut conn = db_connection_config.establish_connection()?;
    let submit_uuid = matches
        .get_one::<String>("submit_uuid")
        .map(|s| uuid::Uuid::parse_str(s.as_ref()))
        .transpose()?
        .unwrap(); // safe by clap
    debug!("Release called for submit: {:?}", submit_uuid);

    let submit = crate::schema::submits::dsl::submits
        .filter(crate::schema::submits::dsl::uuid.eq(submit_uuid))
        .first::<dbmodels::Submit>(&mut conn)?;
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
                query.load::<dbmodels::Artifact>(&mut conn)?
            }
            (Some(name), None) => {
                let query = sel.filter(crate::schema::packages::name.eq(name));
                debug!(
                    "Query: {:?}",
                    diesel::debug_query::<diesel::pg::Pg, _>(&query)
                );
                query.load::<dbmodels::Artifact>(&mut conn)?
            }
            (None, Some(vers)) => {
                let query = sel.filter(crate::schema::packages::version.like(vers));
                debug!(
                    "Query: {:?}",
                    diesel::debug_query::<diesel::pg::Pg, _>(&query)
                );
                query.load::<dbmodels::Artifact>(&mut conn)?
            }
            (None, None) => {
                debug!(
                    "Query: {:?}",
                    diesel::debug_query::<diesel::pg::Pg, _>(&sel)
                );
                sel.load::<dbmodels::Artifact>(&mut conn)?
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

    let release_store = crate::db::models::ReleaseStore::create(&mut conn, release_store_name)?;
    let do_update = matches.get_flag("package_do_update");
    let interactive = !matches.get_flag("noninteractive");

    let now = chrono::offset::Local::now().naive_local();
    // TODO: Find a proper solution to resolve: `error: captured variable cannot escape `FnMut` closure body`:
    let conn = Arc::new(Mutex::new(conn));
    let any_err = arts.into_iter()
        .map(|art| async {
            let art = art; // ensure it is moved
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

                if dest_path.exists() {
                    debug!("Removing {} before writing new file to this path", dest_path.display());
                    tokio::fs::remove_file(&dest_path)
                        .await
                        .with_context(|| anyhow!("Removing {} before writing new file to this path", dest_path.display()))?;
                }

                // else !dest_path.exists()
                tokio::fs::copy(&art_path, &dest_path)
                    .await
                    .with_context(|| anyhow!("Copying {} to {}", art_path.display(), dest_path.display()))
                    .map_err(Error::from)
                    .and_then(|_| {
                        debug!("Updating {:?} to set released = true", art);
                        let rel = crate::db::models::Release::create(&mut conn.clone().lock().unwrap(), &art, &now, &release_store)?;
                        debug!("Release object = {:?}", rel);
                        Ok(dest_path)
                    })
            }
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Vec<Result<_>>>()
        .await
        .into_iter()
        .and_then_ok(|dest_path| {
            if print_released_file_pathes {
                writeln!(std::io::stdout(), "{}", dest_path.display()).map_err(Error::from)
            } else {
                Ok(())
            }
        })
        .filter_map(Result::err)
        .inspect(|err| error!("Error: {}", err.to_string()))
        .last()
        .is_some(); // consume iterator completely, if not empty, there was an error

    if any_err {
        Err(anyhow!("Releasing one or more artifacts failed"))
    } else {
        Ok(())
    }
}

pub async fn rm_release(
    db_connection_config: DbConnectionConfig<'_>,
    config: &Configuration,
    matches: &ArgMatches,
) -> Result<()> {
    let release_store_name = matches.get_one::<String>("release_store_name").unwrap(); // safe by clap
    if !(config.releases_directory().exists() && config.releases_directory().is_dir()) {
        return Err(anyhow!(
            "Release directory does not exist or does not point to directory: {}",
            config.releases_directory().display()
        ));
    }
    if !config.release_stores().contains(release_store_name) {
        return Err(anyhow!("Unknown release store name: {}", release_store_name))
    }

    let pname = matches.get_one::<String>("package_name").unwrap(); // safe by clap
    let pvers = matches.get_one::<String>("package_version").unwrap(); // safe by clap
    debug!("Remove Release called for: {:?} {:?}", pname, pvers);

    let mut conn = db_connection_config.establish_connection()?;

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
        .first::<(crate::db::models::Release, crate::db::models::Artifact)>(&mut conn)?;

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

    diesel::delete(&release).execute(&mut conn)?;
    info!("Release deleted from database");

    Ok(())
}

