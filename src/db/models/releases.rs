//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::Error;
use anyhow::Result;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::PgConnection;

use crate::db::models::Artifact;
use crate::schema::releases;
use crate::schema::releases::*;

#[derive(Debug, Identifiable, Queryable, Associations)]
#[belongs_to(Artifact)]
pub struct Release {
    pub id: i32,
    pub artifact_id: i32,
    pub release_date: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name = "releases"]
struct NewRelease<'a> {
    pub artifact_id: i32,
    pub release_date: &'a NaiveDateTime,
}

impl Release {
    pub fn create<'a>(
        database_connection: &PgConnection,
        art: &Artifact,
        date: &'a NaiveDateTime,
    ) -> Result<Release> {
        let new_rel = NewRelease {
            artifact_id: art.id,
            release_date: date,
        };

        diesel::insert_into(releases::table)
            .values(&new_rel)
            .execute(database_connection)?;

        dsl::releases
            .filter(artifact_id.eq(art.id).and(release_date.eq(date)))
            .first::<Release>(database_connection)
            .map_err(Error::from)
    }
}
