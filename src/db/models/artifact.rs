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
use crate::filestore::path::ArtifactPath;

use anyhow::anyhow;
use anyhow::Error;
use anyhow::Context;
use anyhow::Result;
use diesel::PgConnection;
use diesel::prelude::*;

use crate::db::models::Job;
use crate::schema::artifacts::*;
use crate::schema::artifacts;

#[derive(Debug, Identifiable, Queryable, Associations)]
#[belongs_to(Job)]
pub struct Artifact {
    pub id: i32,
    pub path: String,
    pub released: bool,
    pub job_id: i32,
}

#[derive(Insertable)]
#[table_name="artifacts"]
struct NewArtifact<'a> {
    pub path: &'a str,
    pub released: bool,
    pub job_id: i32,
}

impl Artifact {
    pub fn path_buf(&self) -> PathBuf {
        PathBuf::from(&self.path)
    }

    pub fn create(database_connection: &PgConnection, art_path: &ArtifactPath, art_released: bool, job: &Job) -> Result<Artifact> {
        let path_str = art_path.to_str()
                .ok_or_else(|| anyhow!("Path is not valid UTF-8: {}",  art_path.display()))
                .context("Writing artifact to database")?;
        let new_art = NewArtifact {
            path: path_str,
            released: art_released,
            job_id: job.id,
        };

        diesel::insert_into(artifacts::table)
            .values(&new_art)
            .execute(database_connection)?;

        dsl::artifacts
            .filter(path.eq(path_str).and(job_id.eq(job.id)))
            .first::<Artifact>(database_connection)
            .map_err(Error::from)
    }
}

