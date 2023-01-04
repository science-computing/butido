//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::ops::Deref;

use anyhow::Error;
use anyhow::Result;
use diesel::prelude::*;
use diesel::PgConnection;

use crate::schema::packages;
use crate::schema::packages::*;

#[derive(Debug, Identifiable, Queryable, Eq, PartialEq)]
pub struct Package {
    pub id: i32,
    pub name: String,
    pub version: String,
}

#[derive(Insertable)]
#[diesel(table_name = packages)]
struct NewPackage<'a> {
    pub name: &'a str,
    pub version: &'a str,
}

impl Package {
    pub fn create_or_fetch(
        database_connection: &mut PgConnection,
        p: &crate::package::Package,
    ) -> Result<Package> {
        let new_package = NewPackage {
            name: p.name().deref(),
            version: p.version().deref(),
        };

        database_connection.transaction::<_, Error, _>(|conn| {
            diesel::insert_into(packages::table)
                .values(&new_package)
                .on_conflict_do_nothing()
                .execute(conn)?;

            dsl::packages
                .filter({
                    let p_name = p.name().deref();
                    let p_vers = p.version().deref();

                    name.eq(p_name).and(version.eq(p_vers))
                })
                .first::<Package>(conn)
                .map_err(Error::from)
        })
    }

    pub fn fetch_for_job(database_connection: &mut PgConnection, j: &crate::db::models::Job) -> Result<Option<Package>> {
        Self::fetch_by_id(database_connection, j.package_id)
    }

    pub fn fetch_by_id(database_connection: &mut PgConnection, pid: i32) -> Result<Option<Package>> {
        match dsl::packages.filter(id.eq(pid)).first::<Package>(database_connection) {
            Err(diesel::result::Error::NotFound) => Ok(None),
            Err(e) => Err(Error::from(e)),
            Ok(p) => Ok(Some(p)),
        }
    }
}
