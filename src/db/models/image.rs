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
use diesel::prelude::*;
use diesel::PgConnection;

use crate::schema::images;
use crate::schema::images::*;
use crate::util::docker::ImageName;

#[derive(Identifiable, Queryable)]
pub struct Image {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable)]
#[diesel(table_name = images)]
struct NewImage<'a> {
    pub name: &'a str,
}

impl Image {
    pub fn create_or_fetch(
        database_connection: &mut PgConnection,
        image_name: &ImageName,
    ) -> Result<Image> {
        let new_image = NewImage {
            name: image_name.as_ref(),
        };

        database_connection.transaction::<_, Error, _>(|conn| {
            diesel::insert_into(images::table)
                .values(&new_image)
                .on_conflict_do_nothing()
                .execute(conn)?;

            dsl::images
                .filter(name.eq(image_name.as_ref()))
                .first::<Image>(conn)
                .map_err(Error::from)
        })
    }

    pub fn fetch_for_job(database_connection: &mut PgConnection, j: &crate::db::models::Job) -> Result<Option<Image>> {
        Self::fetch_by_id(database_connection, j.image_id)
    }

    pub fn fetch_by_id(database_connection: &mut PgConnection, iid: i32) -> Result<Option<Image>> {
        match dsl::images.filter(id.eq(iid)).first::<Image>(database_connection) {
            Err(diesel::result::Error::NotFound) => Ok(None),
            Err(e) => Err(Error::from(e)),
            Ok(i) => Ok(Some(i)),
        }
    }
}
