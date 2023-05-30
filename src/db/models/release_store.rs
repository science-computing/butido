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
use diesel::Connection;
use diesel::ExpressionMethods;
use diesel::PgConnection;
use diesel::QueryDsl;
use diesel::RunQueryDsl;

use crate::schema::release_stores;
use crate::schema;

#[derive(Debug, Identifiable, Queryable)]
#[diesel(table_name = release_stores)]
pub struct ReleaseStore {
    pub id: i32,
    pub store_name: String,
}

#[derive(Insertable)]
#[diesel(table_name = release_stores)]
struct NewReleaseStore<'a> {
    pub store_name : &'a str,
}

impl ReleaseStore {
    pub fn create(database_connection: &mut PgConnection, name: &str) -> Result<ReleaseStore> {
        let new_relstore = NewReleaseStore {
            store_name: name,
        };

        database_connection.transaction::<_, Error, _>(|conn| {
            diesel::insert_into(schema::release_stores::table)
                .values(&new_relstore)
                .on_conflict_do_nothing()
                .execute(conn)?;

            schema::release_stores::table
                .filter(schema::release_stores::store_name.eq(name))
                .first::<ReleaseStore>(conn)
                .map_err(Error::from)
        })
    }
}

