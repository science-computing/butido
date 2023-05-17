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
use clap::ArgMatches;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use getset::Getters;
use tracing::debug;

use crate::config::Configuration;

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
            database_host: cli.get_one::<String>("database_host").unwrap_or_else(|| config.database_host()),
            database_port: {
                cli.get_one::<String>("database_port")
                    .map(|s| s.parse::<u16>())
                    .transpose()?
                    .unwrap_or_else(|| *config.database_port())
            },
            database_user: cli.get_one::<String>("database_user").unwrap_or_else(|| config.database_user()),
            database_password: cli.get_one::<String>("database_password").unwrap_or_else(|| config.database_password()),
            database_name: cli.get_one::<String>("database_name").unwrap_or_else(|| config.database_name()),
            database_connection_timeout: {
                cli.get_one::<String>("database_connection_timeout")
                    .map(|s| s.parse::<u16>())
                    .transpose()?
                    .unwrap_or_else( || {
                        // hardcoded default of 30 seconds database timeout
                        config.database_connection_timeout().unwrap_or(30)
                    })
            },
        })
    }

    fn get_database_uri(self) -> String {
        format!(
            "postgres://{user}:{password}@{host}:{port}/{name}?connect_timeout={timeout}",
            host = self.database_host,
            port = self.database_port,
            user = self.database_user,
            password = self.database_password,
            name = self.database_name,
            timeout = self.database_connection_timeout,
        )
    }

    pub fn establish_connection(self) -> Result<PgConnection> {
        debug!("Trying to connect to database: {:?}", self);
        PgConnection::establish(&self.get_database_uri()).map_err(Error::from)
    }

    pub fn establish_pool(self) -> Result<Pool<ConnectionManager<PgConnection>>> {
        debug!("Trying to create a connection pool for database: {:?}", self);
        let manager = ConnectionManager::<PgConnection>::new(self.get_database_uri());
        Pool::builder()
            .min_idle(Some(1))
            .build(manager)
            .map_err(Error::from)
    }

}

