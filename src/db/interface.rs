use std::path::PathBuf;
use std::fmt::Display;

use clap_v3 as clap;
use clap::ArgMatches;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;

use crate::config::Configuration;
use crate::db::DbConnectionConfig;

pub fn interface(db_connection_config: DbConnectionConfig, matches: &ArgMatches, config: &Configuration) -> Result<()> {
    match matches.subcommand() {
        ("cli", Some(matches))        => cli(db_connection_config, matches, config),
        ("artifacts", Some(matches))  => artifacts(db_connection_config),
        ("envvars", Some(matches))    => envvars(db_connection_config),
        (other, _) => return Err(anyhow!("Unknown subcommand: {}", other)),
    }
}

fn cli(db_connection_config: DbConnectionConfig, matches: &ArgMatches, config: &Configuration) -> Result<()> {
    use std::process::Command;

    trait PgCliCommand {
        fn run_for_uri(&self, dbcc: DbConnectionConfig)  -> Result<()>;
    }

    struct Psql(PathBuf);
    impl PgCliCommand for Psql {
        fn run_for_uri(&self, dbcc: DbConnectionConfig)  -> Result<()> {
            Command::new(&self.0)
                .arg(format!("--dbname={}", dbcc.database_name()))
                .arg(format!("--host={}", dbcc.database_host()))
                .arg(format!("--port={}", dbcc.database_port()))
                .arg(format!("--username={}", dbcc.database_user()))
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .output()
                .map_err(Error::from)
                .and_then(|out| {
                    if out.status.success() {
                        info!("pgcli exited successfully");
                        Ok(())
                    } else {
                        Err(anyhow!("gpcli did not exit successfully"))
                            .with_context(|| {
                                match String::from_utf8(out.stderr) {
                                    Ok(log) => anyhow!("{}", log),
                                    Err(e)  => anyhow!("Cannot parse log into valid UTF-8: {}", e),
                                }
                            })
                            .map_err(Error::from)
                    }
                })
        }
    }

    struct PgCli(PathBuf);
    impl PgCliCommand for PgCli {
        fn run_for_uri(&self, dbcc: DbConnectionConfig)  -> Result<()> {
            Command::new(&self.0)
                .arg("--host")
                .arg(dbcc.database_host())
                .arg("--port")
                .arg(dbcc.database_port())
                .arg("--username")
                .arg(dbcc.database_user())
                .arg(dbcc.database_name())
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .output()
                .map_err(Error::from)
                .and_then(|out| {
                    if out.status.success() {
                        info!("pgcli exited successfully");
                        Ok(())
                    } else {
                        Err(anyhow!("gpcli did not exit successfully"))
                            .with_context(|| {
                                match String::from_utf8(out.stderr) {
                                    Ok(log) => anyhow!("{}", log),
                                    Err(e)  => anyhow!("Cannot parse log into valid UTF-8: {}", e),
                                }
                            })
                            .map_err(Error::from)
                    }
                })

        }
    }


    matches.value_of("tool")
        .map(|s| vec![s])
        .unwrap_or_else(|| vec!["psql", "pgcli"])
        .into_iter()
        .filter_map(|s| which::which(&s).ok().map(|path| (path, s)))
        .map(|(path, s)| {
            match s {
                "psql"  => Ok(Box::new(Psql(path)) as Box<dyn PgCliCommand>),
                "pgcli" => Ok(Box::new(PgCli(path)) as Box<dyn PgCliCommand>),
                prog    => Err(anyhow!("Unsupported pg CLI program: {}", prog)),
            }
        })
        .next()
        .transpose()?
        .ok_or_else(|| anyhow!("No Program found"))?
        .run_for_uri(db_connection_config)
}

fn artifacts(conn_cfg: DbConnectionConfig) -> Result<()> {
    use diesel::RunQueryDsl;
    use crate::schema::artifacts::dsl;
    use crate::db::models;

    let hdrs = vec![
        {
            let mut column = ascii_table::Column::default();
            column.header  = "Id".into();
            column.align   = ascii_table::Align::Left;
            column
        },
        {
            let mut column = ascii_table::Column::default();
            column.header  = "Path".into();
            column.align   = ascii_table::Align::Left;
            column
        }
    ];

    let connection = crate::db::establish_connection(conn_cfg)?;
    let data = dsl::artifacts
        .load::<models::Artifact>(&connection)?
        .into_iter()
        .map(|artifact| {
            vec![format!("{}", artifact.id), artifact.path]
        })
        .collect::<Vec<_>>();

    if data.is_empty() {
        info!("No artifacts in database");
    } else {
        display_as_table(hdrs, data);
    }

    Ok(())
}

fn envvars(conn_cfg: DbConnectionConfig) -> Result<()> {
    use diesel::RunQueryDsl;
    use crate::schema::envvars::dsl;
    use crate::db::models;

    let hdrs = vec![
        {
            let mut column = ascii_table::Column::default();
            column.header  = "Id".into();
            column.align   = ascii_table::Align::Left;
            column
        },
        {
            let mut column = ascii_table::Column::default();
            column.header  = "Name".into();
            column.align   = ascii_table::Align::Left;
            column
        },
        {
            let mut column = ascii_table::Column::default();
            column.header  = "Value".into();
            column.align   = ascii_table::Align::Left;
            column
        }
    ];

    let connection = crate::db::establish_connection(conn_cfg)?;
    let data = dsl::envvars
        .load::<models::EnvVar>(&connection)?
        .into_iter()
        .map(|evar| {
            vec![format!("{}", evar.id), evar.name, evar.value]
        })
        .collect::<Vec<_>>();

    if data.is_empty() {
        info!("No environment variables in database");
    } else {
        display_as_table(hdrs, data);
    }

    Ok(())
}

fn display_as_table<D: Display>(headers: Vec<ascii_table::Column>, data: Vec<Vec<D>>) {
    let mut ascii_table = ascii_table::AsciiTable::default();

    ascii_table.max_width = terminal_size::terminal_size()
        .map(|tpl| tpl.0.0 as usize) // an ugly interface indeed!
        .unwrap_or(80);

    headers.into_iter()
        .enumerate()
        .for_each(|(i, c)| {
            ascii_table.columns.insert(i, c);
        });

    ascii_table.print(data);
}

