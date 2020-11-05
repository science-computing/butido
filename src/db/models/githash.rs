use std::ops::Deref;

use anyhow::Error;
use anyhow::Result;
use diesel::PgConnection;
use diesel::prelude::*;

use crate::schema::githashes::*;
use crate::schema::githashes;

#[derive(Queryable)]
pub struct GitHash {
    pub id: i32,
    pub hash: String,
}

#[derive(Insertable)]
#[table_name="githashes"]
struct NewGitHash<'a> {
    pub hash: &'a str,
}

impl GitHash {
    pub fn create_or_fetch(database_connection: &PgConnection, githash: &str) -> Result<GitHash> {
        let new_hash = NewGitHash { hash: githash };

        diesel::insert_into(githashes::table)
            .values(&new_hash)
            .on_conflict_do_nothing()
            .execute(database_connection)?;

        dsl::githashes
            .filter(hash.eq(githash))
            .first::<GitHash>(database_connection)
            .map_err(Error::from)
    }
}

