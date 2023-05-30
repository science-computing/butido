//
// Copyright (c) 2020-2022 science+computing ag and other contributors
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
use crate::db::models::ReleaseStore;
use crate::schema::releases;
use crate::schema::releases::*;

#[derive(Debug, Identifiable, Queryable, Associations)]
#[diesel(belongs_to(Artifact))]
#[diesel(belongs_to(ReleaseStore))]
pub struct Release {
    pub id: i32,
    pub artifact_id: i32,
    pub release_date: NaiveDateTime,
    pub release_store_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = releases)]
struct NewRelease<'a> {
    pub artifact_id: i32,
    pub release_date: &'a NaiveDateTime,
    pub release_store_id: i32,
}

impl Release {
    pub fn create<'a>(
        database_connection: &mut PgConnection,
        art: &Artifact,
        date: &'a NaiveDateTime,
        store: &'a ReleaseStore,
    ) -> Result<Release> {
        let new_rel = NewRelease {
            artifact_id: art.id,
            release_date: date,
            release_store_id: store.id,
        };

        database_connection.transaction::<_, Error, _>(|conn| {
            diesel::insert_into(releases::table)
                .values(&new_rel)
                .execute(conn)?;

            dsl::releases
                .filter(artifact_id.eq(art.id).and(release_date.eq(date)))
                .first::<Release>(conn)
                .map_err(Error::from)
        })
    }
}
