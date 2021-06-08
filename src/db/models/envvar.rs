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

use crate::schema::envvars;
use crate::schema::envvars::*;
use crate::util::EnvironmentVariableName;

#[derive(Debug, Identifiable, Queryable)]
#[table_name = "envvars"]
pub struct EnvVar {
    pub id: i32,
    pub name: String,
    pub value: String,
}

#[derive(Insertable)]
#[table_name = "envvars"]
struct NewEnvVar<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

impl EnvVar {
    pub fn create_or_fetch(
        database_connection: &PgConnection,
        k: &EnvironmentVariableName,
        v: &str,
    ) -> Result<EnvVar> {
        let new_envvar = NewEnvVar {
            name: k.as_ref(),
            value: v,
        };

        database_connection.transaction::<_, Error, _>(|| {
            diesel::insert_into(envvars::table)
                .values(&new_envvar)
                .on_conflict_do_nothing()
                .execute(database_connection)?;

            dsl::envvars
                .filter(name.eq(k.as_ref()).and(value.eq(v)))
                .first::<EnvVar>(database_connection)
                .map_err(Error::from)
        })
    }
}
