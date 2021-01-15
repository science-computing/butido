//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::fmt::Display;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use clap::ArgMatches;
use colored::Colorize;
use diesel::BelongingToDsl;
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use itertools::Itertools;
use log::info;

use crate::config::Configuration;
use crate::db::DbConnectionConfig;
use crate::db::models;
use crate::log::LogItem;
use crate::package::Script;
use crate::schema;

pub fn db(db_connection_config: DbConnectionConfig, config: &Configuration, matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("cli", matches))        => cli(db_connection_config, matches),
        Some(("artifacts", matches))  => artifacts(db_connection_config, matches),
        Some(("envvars", matches))    => envvars(db_connection_config, matches),
        Some(("images", matches))     => images(db_connection_config, matches),
        Some(("submits", matches))    => submits(db_connection_config, matches),
        Some(("jobs", matches))       => jobs(db_connection_config, matches),
        Some(("job", matches))        => job(db_connection_config, config, matches),
        Some((other, _)) => return Err(anyhow!("Unknown subcommand: {}", other)),
        None             => return Err(anyhow!("No subcommand")),
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
    let hdrs = mk_header(vec!["id", "path", "released", "job id"]);
    let conn = crate::db::establish_connection(conn_cfg)?;
    let data = matches.value_of("job_uuid")
        .map(uuid::Uuid::parse_str)
        .transpose()?
        .map(|job_uuid| -> Result<_> {
            dsl::artifacts
                .inner_join(schema::jobs::table)
                .filter(schema::jobs::dsl::uuid.eq(job_uuid))
                .load::<(models::Artifact, models::Job)>(&conn)
                .map_err(Error::from)
        })
        .unwrap_or_else(|| {
            dsl::artifacts
                .inner_join(schema::jobs::table)
                .load::<(models::Artifact, models::Job)>(&conn)
                .map_err(Error::from)
        })?
        .into_iter()
        .map(|(artifact, job)| vec![format!("{}", artifact.id), artifact.path, artifact.released.to_string(), job.uuid.to_string()])
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
    let csv  = matches.is_present("csv");
    let hdrs = mk_header(vec!["id", "time", "uuid"]);
    let conn = crate::db::establish_connection(conn_cfg)?;

    // Helper to map Submit -> Vec<String>
    let submit_to_vec = |submit: models::Submit| {
        vec![format!("{}", submit.id), submit.submit_time.to_string(), submit.uuid.to_string()]
    };

    // Helper to get all submits that were made _for_ a package
    let submits_for = |pkgname: &str| {
        schema::submits::table
            .inner_join(schema::packages::table)
            .filter(schema::packages::dsl::name.eq(&pkgname))
            .select(schema::submits::all_columns)
            .load::<models::Submit>(&conn)
    };

    let data = if let Some(pkgname) = matches.value_of("with_pkg").map(String::from) {
        // Get all submits which included the package, but were not made _for_ the package
        let submits_with_pkg = schema::packages::table
            .filter(schema::packages::name.eq(&pkgname))
            .inner_join(schema::jobs::table.inner_join(schema::submits::table))
            .select(schema::submits::all_columns)
            .load::<models::Submit>(&conn)?;

        let submits_for_pkg = submits_for(&pkgname)?;

        submits_with_pkg
            .into_iter()
            .chain(submits_for_pkg.into_iter())
            .map(submit_to_vec)
            .collect::<Vec<_>>()
    } else {
        if let Some(pkgname) = matches.value_of("for_pkg") {
            // Get all submits _for_ the package
            submits_for(pkgname)?
                .into_iter()
                .map(submit_to_vec)
                .collect::<Vec<_>>()
        } else {
            // default: Get all submits
            schema::submits::table
                .load::<models::Submit>(&conn)?
                .into_iter()
                .map(submit_to_vec)
                .collect::<Vec<_>>()
        }
    };

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
    let hdrs = mk_header(vec!["id", "submit uuid", "job uuid", "time", "endpoint", "success", "package", "version", "Env Name", "Env Value"]);
    let conn = crate::db::establish_connection(conn_cfg)?;
    let jobs = matches.value_of("submit_uuid")
        .map(uuid::Uuid::parse_str)
        .transpose()?
        .map(|submit_uuid| {
            let sel = schema::jobs::table
                .inner_join(schema::submits::table)
                .inner_join(schema::endpoints::table)
                .inner_join(schema::packages::table)
                .left_outer_join(schema::job_envs::table.inner_join(schema::envvars::table))
                .filter(schema::submits::uuid.eq(&submit_uuid));

            if let Some((env_name, env_value)) = matches.value_of("filter_env").map(crate::util::env::parse_to_env).transpose()? {
                sel.filter({
                        use crate::diesel::BoolExpressionMethods;

                        schema::envvars::dsl::name
                            .eq(env_name.as_ref())
                            .and(schema::envvars::dsl::value.eq(env_value))
                    })
                    .load::<(models::Job, models::Submit, models::Endpoint, models::Package, Option<(models::JobEnv, models::EnvVar)>)>(&conn)
                    .map_err(Error::from)
            } else {
                sel.load::<(models::Job, models::Submit, models::Endpoint, models::Package, Option<(models::JobEnv, models::EnvVar)>)>(&conn)
                    .map_err(Error::from)
            }
        })
        .unwrap_or_else(|| {
            dsl::jobs
                .inner_join(crate::schema::submits::table)
                .inner_join(crate::schema::endpoints::table)
                .inner_join(crate::schema::packages::table)
                .left_outer_join(schema::job_envs::table.inner_join(schema::envvars::table))
                .load::<(models::Job, models::Submit, models::Endpoint, models::Package, Option<(models::JobEnv, models::EnvVar)>)>(&conn)
                .map_err(Error::from)
        })?;

    let data = jobs.into_iter()
        .map(|(job, submit, ep, package, o_env)| {
            let success = crate::log::ParsedLog::build_from(&job.log_text)?
                .is_successfull()
                .map(|b| if b { "yes" } else { "no" })
                .map(String::from)
                .unwrap_or_else(|| String::from("unknown"));

            let env = o_env.map(|tpl| (tpl.1.name, tpl.1.value)).unwrap_or_default();
            Ok(vec![format!("{}", job.id), submit.uuid.to_string(), job.uuid.to_string(), submit.submit_time.to_string(), ep.name, success, package.name, package.version, env.0, env.1])
        })
        .collect::<Result<Vec<_>>>()?;

    if data.is_empty() {
        info!("No submits in database");
    } else {
        display_data(hdrs, data, csv)?;
    }

    Ok(())
}

fn job(conn_cfg: DbConnectionConfig, config: &Configuration, matches: &ArgMatches) -> Result<()> {
    let script_highlight      = !matches.is_present("no_script_highlight");
    let script_line_numbers   = !matches.is_present("no_script_line_numbers");
    let configured_theme      = config.script_highlight_theme();
    let show_log              = matches.is_present("show_log");
    let show_script           = matches.is_present("show_script");
    let csv                   = matches.is_present("csv");
    let conn                  = crate::db::establish_connection(conn_cfg)?;
    let job_uuid              = matches.value_of("job_uuid")
        .map(uuid::Uuid::parse_str)
        .transpose()?
        .unwrap();

    let data = schema::jobs::table
        .filter(schema::jobs::dsl::uuid.eq(job_uuid))
        .inner_join(schema::submits::table)
        .inner_join(schema::endpoints::table)
        .inner_join(schema::packages::table)
        .inner_join(schema::images::table)
        .first::<(models::Job, models::Submit, models::Endpoint, models::Package, models::Image)>(&conn)?;

    let parsed_log = crate::log::ParsedLog::build_from(&data.0.log_text)?;
    let success = parsed_log.is_successfull();

    if csv {
        let hdrs = mk_header(vec!["UUID", "success", "Package Name", "Package Version", "Ran on", "Image Name", "Container"]);

        let data = vec![
            vec![
                data.0.uuid.to_string(),
                String::from(match success {
                    Some(true)  => "yes",
                    Some(false) => "no",
                    None        => "unknown",
                }),
                data.3.name.to_string(),
                data.3.version.to_string(),
                data.2.name.to_string(),
                data.4.name.to_string(),
                data.0.container_hash.to_string(),
            ]
        ];
        display_data(hdrs, data, csv)
    } else {
        let env_vars = if matches.is_present("show_env") {
            Some({
                models::JobEnv::belonging_to(&data.0)
                    .inner_join(schema::envvars::table)
                    .load::<(models::JobEnv, models::EnvVar)>(&conn)?
                    .into_iter()
                    .map(|tpl| tpl.1)
                    .enumerate()
                    .map(|(i, env)| format!("\t{:>3}. {}={}", i, env.name, env.value))
                    .join("\n")
            })
        } else {
            None
        };

        let mut out = std::io::stdout();
        let s = indoc::formatdoc!(r#"
                Job:        {job_uuid}
                Submit:     {submit_uuid}
                Succeeded:  {succeeded}
                Package:    {package_name} {package_version}

                Ran on:     {endpoint_name}
                Image:      {image_name}
                Container:  {container_hash}

                Script:     {script_len} lines
                Log:        {log_len} lines

            "#,

            job_uuid = match success {
                Some(true) => data.0.uuid.to_string().green(),
                Some(false) => data.0.uuid.to_string().red(),
                None => data.0.uuid.to_string().cyan(),
            },

            submit_uuid = data.1.uuid.to_string().cyan(),

            succeeded = match success {
                Some(true) => String::from("yes").green(),
                Some(false) => String::from("no").red(),
                None => String::from("unknown").cyan(),
            },
            package_name    = data.3.name.cyan(),
            package_version = data.3.version.cyan(),
            endpoint_name   = data.2.name.cyan(),
            image_name      = data.4.name.cyan(),
            container_hash  = data.0.container_hash.cyan(),
            script_len      = format!("{:<4}", data.0.script_text.lines().count()).cyan(),
            log_len         = format!("{:<4}", data.0.log_text.lines().count()).cyan(),
        );
        let _ = writeln!(out, "{}", s)?;

        if let Some(envs) = env_vars {
            let s = indoc::formatdoc!(r#"
                ---

                {envs}

            "#, envs = envs);
            let _ = writeln!(out, "{}", s)?;
        }

        if show_script {
            let theme = configured_theme
                .as_ref()
                .ok_or_else(|| anyhow!("Highlighting for script enabled, but no theme configured"))?;
            let script = Script::from(data.0.script_text);
            let script = crate::ui::script_to_printable(&script, script_highlight, theme, script_line_numbers)?;

            let s = indoc::formatdoc!(r#"
                ---

                {script}

            "#, script = script);
            let _ = writeln!(out, "{}", s)?;
        }

        if show_log {
            let log = parsed_log.iter()
                .map(|line_item| match line_item {
                    LogItem::Line(s)         => Ok(String::from_utf8(s.to_vec())?.normal()),
                    LogItem::Progress(u)     => Ok(format!("#BUTIDO:PROGRESS:{}", u).bright_black()),
                    LogItem::CurrentPhase(p) => Ok(format!("#BUTIDO:PHASE:{}", p).bright_black()),
                    LogItem::State(Ok(()))   => Ok(format!("#BUTIDO:STATE:OK").green()),
                    LogItem::State(Err(s))   => Ok(format!("#BUTIDO:STATE:ERR:{}", s).red()),
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter() // ugly, but hey... not important right now.
                .join("\n");

            let s = indoc::formatdoc!(r#"
                ---

                {log}

            "#, log = log);
            let _ = writeln!(out, "{}", s)?;
        }

        Ok(())
    }
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

