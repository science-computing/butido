use std::ops::Deref;

use anyhow::Error;
use anyhow::Result;
use diesel::PgConnection;
use diesel::prelude::*;

use crate::schema::packages::*;
use crate::schema::packages;

#[derive(Identifiable, Queryable)]
pub struct Package {
    pub id: i32,
    pub name: String,
    pub version: String,
}

#[derive(Insertable)]
#[table_name="packages"]
struct NewPackage<'a> {
    pub name: &'a str,
    pub version: &'a str,
}

impl Package {
    pub fn create_or_fetch(database_connection: &PgConnection, p: &crate::package::Package) -> Result<Package> {
        let new_package = NewPackage {
            name:    p.name().deref(),
            version: p.version().deref(),
        };

        diesel::insert_into(packages::table)
            .values(&new_package)
            .on_conflict_do_nothing()
            .execute(database_connection)?;

        dsl::packages
            .filter({
                let p_name = p.name().deref();
                let p_vers = p.version().deref();

                name.eq(p_name).and(version.eq(p_vers))
            })
            .first::<Package>(database_connection)
            .map_err(Error::from)
    }
}

