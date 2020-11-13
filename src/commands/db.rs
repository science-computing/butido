use std::fmt::Display;
use std::path::PathBuf;
use std::process::Command;

use clap_v3 as clap;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use clap::ArgMatches;
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use itertools::Itertools;

use crate::db::DbConnectionConfig;
use crate::db::models;

pub fn db(db_connection_config: DbConnectionConfig, matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        ("cli", Some(matches))        => cli(db_connection_config, matches),
        ("artifacts", Some(matches))  => artifacts(db_connection_config, matches),
        ("envvars", Some(matches))    => envvars(db_connection_config, matches),
        ("images", Some(matches))     => images(db_connection_config, matches),
        ("submits", Some(matches))    => submits(db_connection_config, matches),
        ("jobs", Some(matches))       => jobs(db_connection_config, matches),
        (other, _) => return Err(anyhow!("Unknown subcommand: {}", other)),
    }
}

fn cli(db_connection_config: DbConnectionConfig, matches: &ArgMatches) -> Result<()> {
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

fn artifacts(conn_cfg: DbConnectionConfig, matches: &ArgMatches) -> Result<()> {
    use crate::schema::artifacts::dsl;

    let csv  = matches.is_present("csv");
    let hdrs = mk_header(vec!["id", "path"]);
    let conn = crate::db::establish_connection(conn_cfg)?;
    let data = dsl::artifacts
        .load::<models::Artifact>(&conn)?
        .into_iter()
        .map(|artifact| vec![format!("{}", artifact.id), artifact.path])
        .collect::<Vec<_>>();

    if data.is_empty() {
        info!("No artifacts in database");
    } else {
        display_data(hdrs, data, csv)?;
    }

    Ok(())
}

fn envvars(conn_cfg: DbConnectionConfig, matches: &ArgMatches) -> Result<()> {
    use crate::schema::envvars::dsl;

    let csv  = matches.is_present("csv");
    let hdrs = mk_header(vec!["id", "name", "value"]);
    let conn = crate::db::establish_connection(conn_cfg)?;
    let data = dsl::envvars
        .load::<models::EnvVar>(&conn)?
        .into_iter()
        .map(|evar| {
            vec![format!("{}", evar.id), evar.name, evar.value]
        })
        .collect::<Vec<_>>();

    if data.is_empty() {
        info!("No environment variables in database");
    } else {
        display_data(hdrs, data, csv)?;
    }

    Ok(())
}

fn images(conn_cfg: DbConnectionConfig, matches: &ArgMatches) -> Result<()> {
    use crate::schema::images::dsl;

    let csv  = matches.is_present("csv");
    let hdrs = mk_header(vec!["id", "name"]);
    let conn = crate::db::establish_connection(conn_cfg)?;
    let data = dsl::images
        .load::<models::Image>(&conn)?
        .into_iter()
        .map(|image| {
            vec![format!("{}", image.id), image.name]
        })
        .collect::<Vec<_>>();

    if data.is_empty() {
        info!("No images in database");
    } else {
        display_data(hdrs, data, csv)?;
    }

    Ok(())
}

fn submits(conn_cfg: DbConnectionConfig, matches: &ArgMatches) -> Result<()> {
    use crate::schema::submits::dsl;

    let csv  = matches.is_present("csv");
    let hdrs = mk_header(vec!["id", "time", "uuid"]);
    let conn = crate::db::establish_connection(conn_cfg)?;
    let data = dsl::submits
        .load::<models::Submit>(&conn)?
        .into_iter()
        .map(|submit| {
            vec![format!("{}", submit.id), submit.submit_time.to_string(), submit.uuid.to_string()]
        })
        .collect::<Vec<_>>();

    if data.is_empty() {
        info!("No submits in database");
    } else {
        display_data(hdrs, data, csv)?;
    }

    Ok(())
}

fn jobs(conn_cfg: DbConnectionConfig, matches: &ArgMatches) -> Result<()> {
    use crate::schema::jobs::dsl;

    let csv  = matches.is_present("csv");
    let hdrs = mk_header(vec!["id", "submit uuid", "time", "endpoint", "success"]);
    let conn = crate::db::establish_connection(conn_cfg)?;
    let data = dsl::jobs
        .load::<models::Job>(&conn)?
        .into_iter()
        .map(|job| {
            let submit = crate::schema::submits::dsl::submits
                .filter(crate::schema::submits::dsl::id.eq(job.id))
                .first::<models::Submit>(&conn)?;

            let ep = crate::schema::endpoints::dsl::endpoints
                .filter(crate::schema::endpoints::dsl::id.eq(job.endpoint_id))
                .first::<models::Endpoint>(&conn)?;

            let success = crate::log::log_is_successfull(&job.log_text)?
                .map(|b| if b {
                    String::from("yes")
                } else {
                    String::from("no")
                })
                .unwrap_or_else(|| String::from("unknown"));

            Ok(vec![format!("{}", job.id), submit.uuid.to_string(), submit.submit_time.to_string(), ep.name, success])
        })
        .collect::<Result<Vec<_>>>()?;

    if data.is_empty() {
        info!("No submits in database");
    } else {
        display_data(hdrs, data, csv)?;
    }

    Ok(())
}


fn mk_header(vec: Vec<&str>) -> Vec<ascii_table::Column> {
    vec.into_iter()
        .map(|name| {
            let mut column = ascii_table::Column::default();
            column.header  = name.into();
            column.align   = ascii_table::Align::Left;
            column
        })
        .collect()
}

/// Display the passed data as nice ascii table,
/// or, if stdout is a pipe, print it nicely parseable
fn display_data<D: Display>(headers: Vec<ascii_table::Column>, data: Vec<Vec<D>>, csv: bool) -> Result<()> {
    use std::io::Write;

    if csv {
         use csv::WriterBuilder;
         let mut wtr = WriterBuilder::new().from_writer(vec![]);
         for record in data.into_iter() {
             let r: Vec<String> = record.into_iter()
                 .map(|e| e.to_string())
                 .collect();

             wtr.write_record(&r)?;
         }

        let out = std::io::stdout();
        let mut lock = out.lock();

         wtr.into_inner()
             .map_err(Error::from)
             .and_then(|t| String::from_utf8(t).map_err(Error::from))
             .and_then(|text| writeln!(lock, "{}", text).map_err(Error::from))

    } else {
        if atty::is(atty::Stream::Stdout) {
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
            Ok(())
        } else {
            let out = std::io::stdout();
            let mut lock = out.lock();
            for list in data {
                writeln!(lock, "{}", list.iter().map(|d| d.to_string()).join(" "))?;
            }
            Ok(())
        }
    }
}

