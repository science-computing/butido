//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::str::FromStr;

use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use getset::Getters;
use log::debug;

use crate::config::Configuration;
use crate::util::progress::ProgressBars;

#[derive(Getters)]
pub struct DbConnectionConfig<'a> {
    #[getset(get = "pub")]
    database_host: &'a str,

    #[getset(get = "pub")]
    database_port: u16,

    #[getset(get = "pub")]
    database_user: &'a str,

    #[getset(get = "pub")]
    database_password: &'a str,

    #[getset(get = "pub")]
    database_name: &'a str,

    #[getset(get = "pub")]
    database_connection_timeout: u16,
}

impl<'a> std::fmt::Debug for DbConnectionConfig<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "postgres://{user}:PASSWORD@{host}:{port}/{name}?connect_timeout={timeout}",
            host = self.database_host,
            port = self.database_port,
            user = self.database_user,
            name = self.database_name,
            timeout = self.database_connection_timeout
        )
    }
}

impl<'a> DbConnectionConfig<'a> {
    pub fn parse(config: &'a Configuration, cli: &'a ArgMatches) -> Result<DbConnectionConfig<'a>> {
        Ok(DbConnectionConfig {
            database_host: cli.value_of("database_host").unwrap_or_else(|| config.database_host()),
            database_port: {
                cli.value_of("database_port")
                    .map(u16::from_str)
                    .transpose()?
                    .unwrap_or_else(|| *config.database_port())
            },
            database_user: cli.value_of("database_user").unwrap_or_else(|| config.database_user()),
            database_password: cli.value_of("database_password").unwrap_or_else(|| config.database_password()),
            database_name: cli.value_of("database_name").unwrap_or_else(|| config.database_name()),
            database_connection_timeout: {
                cli.value_of("database_connection_timeout")
                    .map(u16::from_str)
                    .transpose()?
                    .unwrap_or_else( || {
                        // hardcoded default of 30 seconds database timeout
                        config.database_connection_timeout().unwrap_or(30)
                    })
            },
        })
    }

    pub fn establish_connection(self, progressbars: &ProgressBars) -> Result<PgConnection> {
        debug!("Trying to connect to database: {:?}", self);
        let database_uri: String = format!(
            "postgres://{user}:{password}@{host}:{port}/{name}?connect_timeout={timeout}",
            host = self.database_host,
            port = self.database_port,
            user = self.database_user,
            password = self.database_password,
            name = self.database_name,
            timeout = self.database_connection_timeout,
        );

        let bar = progressbars.spinner();
        bar.set_message("Establishing database connection");

        let conn = PgConnection::establish(&database_uri).map_err(Error::from);
        if conn.is_err() {
            bar.finish_with_message("Connection could not be established");
        } else {
            bar.finish_with_message("Connection established");
        }
        conn
    }

}

