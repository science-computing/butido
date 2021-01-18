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
#[table_name = "images"]
struct NewImage<'a> {
    pub name: &'a str,
}

impl Image {
    pub fn create_or_fetch(
        database_connection: &PgConnection,
        image_name: &ImageName,
    ) -> Result<Image> {
        let new_image = NewImage {
            name: image_name.as_ref(),
        };

        diesel::insert_into(images::table)
            .values(&new_image)
            .on_conflict_do_nothing()
            .execute(database_connection)?;

        dsl::images
            .filter(name.eq(image_name.as_ref()))
            .first::<Image>(database_connection)
            .map_err(Error::from)
    }
}
