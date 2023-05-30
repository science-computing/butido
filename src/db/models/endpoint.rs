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
use diesel::prelude::*;
use diesel::PgConnection;

use crate::config::EndpointName;
use crate::schema::endpoints;
use crate::schema::endpoints::*;

#[derive(Identifiable, Queryable, Eq, PartialEq)]
pub struct Endpoint {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable)]
#[diesel(table_name = endpoints)]
struct NewEndpoint<'a> {
    pub name: &'a str,
}

impl Endpoint {
    pub fn create_or_fetch(database_connection: &mut PgConnection, ep_name: &EndpointName) -> Result<Endpoint> {
        let new_ep = NewEndpoint { name: ep_name.as_ref() };

        database_connection.transaction::<_, Error, _>(|conn| {
            diesel::insert_into(endpoints::table)
                .values(&new_ep)
                .on_conflict_do_nothing()
                .execute(conn)?;

            dsl::endpoints
                .filter(name.eq(ep_name.as_ref()))
                .first::<Endpoint>(conn)
                .map_err(Error::from)
        })
    }

    pub fn fetch_for_job(database_connection: &mut PgConnection, j: &crate::db::models::Job) -> Result<Option<Endpoint>> {
        Self::fetch_by_id(database_connection, j.endpoint_id)
    }

    pub fn fetch_by_id(database_connection: &mut PgConnection, eid: i32) -> Result<Option<Endpoint>> {
        match dsl::endpoints.filter(id.eq(eid)).first::<Endpoint>(database_connection) {
            Err(diesel::result::Error::NotFound) => Ok(None),
            Err(e) => Err(Error::from(e)),
            Ok(e) => Ok(Some(e)),
        }
    }
}
