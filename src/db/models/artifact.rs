//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use crate::filestore::path::ArtifactPath;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::PgConnection;

use crate::db::models::Job;
use crate::db::models::Release;
use crate::schema::artifacts;
use crate::schema::artifacts::*;

#[derive(Debug, Identifiable, Queryable, Associations)]
#[diesel(belongs_to(Job))]
pub struct Artifact {
    pub id: i32,
    pub path: String,
    pub job_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = artifacts)]
struct NewArtifact<'a> {
    pub path: &'a str,
    pub job_id: i32,
}

impl Artifact {
    pub fn path_buf(&self) -> PathBuf {
        PathBuf::from(&self.path)
    }

    pub fn released(
        self,
        database_connection: &mut PgConnection,
        release_date: &NaiveDateTime,
        release_store_name: &str,
    ) -> Result<crate::db::models::Release> {
        let rs = crate::db::models::ReleaseStore::create(database_connection, release_store_name)?;
        crate::db::models::Release::create(database_connection, &self, release_date, &rs)
    }

    pub fn get_release(&self, database_connection: &mut PgConnection) -> Result<Option<Release>> {
        use crate::schema;

        schema::artifacts::table
            .inner_join(schema::releases::table)
            .filter(schema::releases::artifact_id.eq(self.id))
            .select(schema::releases::all_columns)
            .first::<Release>(database_connection)
            .optional()
            .map_err(Error::from)
    }

    pub fn create(
        database_connection: &mut PgConnection,
        art_path: &ArtifactPath,
        job: &Job,
    ) -> Result<Artifact> {
        let path_str = art_path
            .to_str()
            .ok_or_else(|| anyhow!("Path is not valid UTF-8: {}", art_path.display()))
            .context("Writing artifact to database")?;
        let new_art = NewArtifact {
            path: path_str,
            job_id: job.id,
        };

        database_connection.transaction::<_, Error, _>(|conn| {
            diesel::insert_into(artifacts::table)
                .values(&new_art)
                .execute(conn)?;

            dsl::artifacts
                .filter(path.eq(path_str).and(job_id.eq(job.id)))
                .first::<Artifact>(conn)
                .map_err(Error::from)
        })
    }
}
