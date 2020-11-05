use std::ops::Deref;

use anyhow::Error;
use anyhow::Result;
use diesel::PgConnection;
use diesel::prelude::*;
use chrono::NaiveDateTime;
use diesel::sql_types::Uuid;
use diesel::sql_types::Jsonb;

use crate::schema::submits::*;
use crate::schema::submits;
use crate::db::models::Image;
use crate::db::models::Package;
use crate::db::models::GitHash;

#[derive(Queryable)]
pub struct Submit {
    pub id: i32,
    pub uuid: ::uuid::Uuid,
    pub submit_time: NaiveDateTime,
    pub requested_image_id: i32,
    pub requested_package_id: i32,
    pub repo_hash_id: i32,
    pub tree: serde_json::Value,
}

#[derive(Insertable)]
#[table_name="submits"]
struct NewSubmit<'a> {
    pub uuid: &'a ::uuid::Uuid,
    pub submit_time: &'a NaiveDateTime,
    pub requested_image_id: i32,
    pub requested_package_id: i32,
    pub repo_hash_id: i32,
    pub tree: serde_json::Value,
}

impl Submit {
    pub fn create(database_connection: &PgConnection,
                  t: &crate::package::Tree,
                  submit_datetime: &NaiveDateTime,
                  submit_id: &::uuid::Uuid,
                  requested_image: &Image,
                  requested_package: &Package,
                  repo_hash: &GitHash)
        -> Result<Vec<Submit>>
{
        let tree_json = serde_json::to_value(t)?;

        let new_submit = NewSubmit {
            uuid: submit_id,
            submit_time: submit_datetime,
            requested_image_id: requested_image.id,
            requested_package_id: requested_package.id,
            repo_hash_id: repo_hash.id,
            tree: tree_json,
        };

        diesel::insert_into(submits::table)
            .values(&new_submit)
            .on_conflict_do_nothing()
            .execute(database_connection)?;

        dsl::submits
            .filter(uuid.eq(uuid))
            .limit(1)
            .load::<Submit>(database_connection)
            .map_err(Error::from)
    }
}

