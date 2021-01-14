//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::path::PathBuf;

use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use clap::ArgMatches;
use diesel::prelude::*;
use log::{debug, trace};
use tokio::stream::StreamExt;

use crate::db::models as dbmodels;
use crate::config::Configuration;
use crate::db::DbConnectionConfig;

pub async fn release(db_connection_config: DbConnectionConfig, config: &Configuration, matches: &ArgMatches) -> Result<()> {
    if !(config.releases_directory().exists() && config.releases_directory().is_dir()) {
        return Err(anyhow!("Release directory does not exist or does not point to directory: {}", config.releases_directory().display()))
    }

    let pname = matches.value_of("package_name")
            .map(String::from);

    let pvers = matches
        .value_of("package_version")
        .map(String::from);

    debug!("Release called for: {:?} {:?}", pname, pvers);

    let conn = crate::db::establish_connection(db_connection_config)?;
    let submit_uuid = matches.value_of("submit_uuid")
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
            .inner_join({
                crate::schema::jobs::table
                    .inner_join(crate::schema::packages::table)
            })
            .filter(crate::schema::jobs::submit_id.eq(submit.id))
            .filter(crate::schema::artifacts::released.eq(false));

        match (pname, pvers) {
            (Some(name), Some(vers)) => {
                let query = sel.filter(crate::schema::packages::name.eq(name))
                    .filter(crate::schema::packages::version.like(vers));
                debug!("Query: {}", diesel::debug_query(&query).to_string());
                query.load::<(dbmodels::Artifact, (dbmodels::Job, dbmodels::Package))>(&conn)?
            },
            (Some(name), None) => {
                let query = sel.filter(crate::schema::packages::name.eq(name));
                debug!("Query: {}", diesel::debug_query(&query).to_string());
                query.load::<(dbmodels::Artifact, (dbmodels::Job, dbmodels::Package))>(&conn)?
            },
            (None, Some(vers)) => {
                let query = sel.filter(crate::schema::packages::version.like(vers));
                debug!("Query: {}", diesel::debug_query(&query).to_string());
                query.load::<(dbmodels::Artifact, (dbmodels::Job, dbmodels::Package))>(&conn)?
            },
            (None, None) => {
                debug!("Query: {}", diesel::debug_query(&sel).to_string());
                sel.load::<(dbmodels::Artifact, (dbmodels::Job, dbmodels::Package))>(&conn)?
            },
        }
    };
    debug!("Artifacts = {:?}", arts);

    arts.iter()
        .filter_map(|(art, _)| art.path_buf().parent().map(|p| config.releases_directory().join(p)))
        .map(|p| async {
            debug!("mkdir {:?}", p);
            tokio::fs::create_dir_all(p).await.map_err(Error::from)
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<()>>()
        .await?;

    let staging_base: &PathBuf = &config.staging_directory().join(submit.uuid.to_string());
    arts.into_iter()
        .map(|(art, _)| async move {
            let art_path  = staging_base.join(&art.path);
            let dest_path = config.releases_directory().join(&art.path);
            debug!("Trying to release {} to {}", art_path.display(), dest_path.display());

            if !art_path.is_file() {
                trace!("Artifact does not exist as file, cannot release it: {:?}", art);
                Err(anyhow!("Not a file: {}", art_path.display()))
            } else {
                if dest_path.exists() {
                    Err(anyhow!("Does already exist: {}", dest_path.display()))
                } else {
                    tokio::fs::rename(art_path, dest_path)
                        .await
                        .map_err(Error::from)
                        .map(|_| art)
                }
            }
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<Vec<_>>>()
        .await?
        .into_iter()
        .map(|art| {
            debug!("Updating {:?} to set released = true", art);
            diesel::update(&art)
                .set(crate::schema::artifacts::released.eq(true))
                .execute(&conn)
                .map(|_| ())
                .map_err(Error::from)
        })
        .collect()
}
