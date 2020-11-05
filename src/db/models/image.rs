use std::ops::Deref;

use anyhow::Error;
use anyhow::Result;
use diesel::PgConnection;
use diesel::prelude::*;

use crate::schema::images::*;
use crate::schema::images;

#[derive(Queryable)]
pub struct Image {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable)]
#[table_name="images"]
struct NewImage<'a> {
    pub name: &'a str,
}

impl Image {
    pub fn create_or_fetch(database_connection: &PgConnection, image_name: &str) -> Result<Image> {
        let new_image = NewImage { name: image_name };

        diesel::insert_into(images::table)
            .values(&new_image)
            .on_conflict_do_nothing()
            .execute(database_connection)?;

        dsl::images
            .filter(name.eq(image_name))
            .first::<Image>(database_connection)
            .map_err(Error::from)
    }
}

