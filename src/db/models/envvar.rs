use anyhow::Error;
use anyhow::Result;
use diesel::PgConnection;
use diesel::prelude::*;

use crate::schema::envvars::*;
use crate::schema::envvars;
use crate::util::EnvironmentVariableName;

#[derive(Identifiable, Queryable)]
#[table_name="envvars"]
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
    pub fn create_or_fetch(database_connection: &PgConnection, k: &EnvironmentVariableName, v: &str) -> Result<EnvVar> {
        let new_envvar = NewEnvVar { name: k.as_ref(), value: v };

        diesel::insert_into(envvars::table)
            .values(&new_envvar)
            .on_conflict_do_nothing()
            .execute(database_connection)?;

        dsl::envvars
            .filter(name.eq(k.as_ref()).and(value.eq(v)))
            .first::<EnvVar>(database_connection)
            .map_err(Error::from)
    }
}

