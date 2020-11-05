use std::ops::Deref;

use anyhow::Error;
use anyhow::Result;
use diesel::PgConnection;
use diesel::prelude::*;

use crate::schema::envvars::*;
use crate::schema::envvars;

#[derive(Queryable)]
pub struct EnvVar {
    pub id: i32,
    pub name: String,
    pub value: String,
}

#[derive(Insertable)]
#[table_name="envvars"]
struct NewEnvVar<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

impl EnvVar {
    pub fn create_or_fetch(database_connection: &PgConnection, k: &str, v: &str) -> Result<EnvVar> {
        let new_envvar = NewEnvVar { name: k, value: v };

        diesel::insert_into(envvars::table)
            .values(&new_envvar)
            .on_conflict_do_nothing()
            .execute(database_connection)?;

        dsl::envvars
            .filter(name.eq(k).and(value.eq(v)))
            .first::<EnvVar>(database_connection)
            .map_err(Error::from)
    }
}

