//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::PgConnection;

use crate::db::models::GitHash;
use crate::db::models::Image;
use crate::db::models::Package;
use crate::schema::submits;
use crate::schema::submits::*;

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Associations)]
#[diesel(belongs_to(Package, foreign_key = requested_package_id))]
#[diesel(belongs_to(Image, foreign_key = requested_image_id))]
#[diesel(table_name = submits)]
pub struct Submit {
    pub id: i32,
    pub uuid: ::uuid::Uuid,
    pub submit_time: NaiveDateTime,
    pub requested_image_id: i32,
    pub requested_package_id: i32,
    pub repo_hash_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = submits)]
struct NewSubmit<'a> {
    pub uuid: &'a ::uuid::Uuid,
    pub submit_time: &'a NaiveDateTime,
    pub requested_image_id: i32,
    pub requested_package_id: i32,
    pub repo_hash_id: i32,
}

impl Submit {
    pub fn create(
        database_connection: &mut PgConnection,
        submit_datetime: &NaiveDateTime,
        submit_id: &::uuid::Uuid,
        requested_image: &Image,
        requested_package: &Package,
        repo_hash: &GitHash,
    ) -> Result<Submit> {
        let new_submit = NewSubmit {
            uuid: submit_id,
            submit_time: submit_datetime,
            requested_image_id: requested_image.id,
            requested_package_id: requested_package.id,
            repo_hash_id: repo_hash.id,
        };

        database_connection.transaction::<_, Error, _>(|conn| {
            diesel::insert_into(submits::table)
                .values(&new_submit)

                // required because if we re-use the staging store, we do not create a new UUID but re-use the old one
                .on_conflict_do_nothing()

                .execute(conn)
                .context("Inserting new submit into submits table")?;

            Self::with_id(conn, submit_id)
        })
    }

    pub fn with_id(database_connection: &mut PgConnection, submit_id: &::uuid::Uuid) -> Result<Submit> {
        dsl::submits
            .filter(submits::uuid.eq(submit_id))
            .first::<Submit>(database_connection)
            .context("Loading submit")
            .map_err(Error::from)
    }
}
