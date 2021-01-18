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

use crate::schema::githashes;
use crate::schema::githashes::*;

#[derive(Queryable)]
pub struct GitHash {
    pub id: i32,
    pub hash: String,
}

#[derive(Insertable)]
#[table_name = "githashes"]
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
