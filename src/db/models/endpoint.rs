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

use crate::config::EndpointName;
use crate::schema::endpoints;
use crate::schema::endpoints::*;

#[derive(Identifiable, Queryable, Eq, PartialEq)]
pub struct Endpoint {
    pub id: i32,
    pub name: String,
}

#[derive(Insertable)]
#[table_name = "endpoints"]
struct NewEndpoint<'a> {
    pub name: &'a str,
}

impl Endpoint {
    pub fn create_or_fetch(database_connection: &PgConnection, ep_name: &EndpointName) -> Result<Endpoint> {
        let new_ep = NewEndpoint { name: ep_name.as_ref() };

        diesel::insert_into(endpoints::table)
            .values(&new_ep)
            .on_conflict_do_nothing()
            .execute(database_connection)?;

        dsl::endpoints
            .filter(name.eq(ep_name.as_ref()))
            .first::<Endpoint>(database_connection)
            .map_err(Error::from)
    }
}
