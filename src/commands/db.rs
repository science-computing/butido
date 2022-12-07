//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'db' subcommand

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use clap::ArgMatches;
use colored::Colorize;
use diesel::BelongingToDsl;
use diesel::ExpressionMethods;
use diesel::JoinOnDsl;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use itertools::Itertools;
use log::debug;
use log::info;
use log::trace;

use crate::commands::util::get_date_filter;
use crate::config::Configuration;
use crate::db::models;
use crate::db::DbConnectionConfig;
use crate::log::JobResult;
use crate::package::Script;
use crate::schema;

diesel_migrations::embed_migrations!("migrations");

/// Implementation of the "db" subcommand
pub fn db(
    db_connection_config: DbConnectionConfig<'_>,
    config: &Configuration,
    matches: &ArgMatches,
) -> Result<()> {
    match matches.subcommand() {
        Some(("cli", matches)) => cli(db_connection_config, matches),
        Some(("setup", _matches)) => setup(db_connection_config),
        Some(("artifacts", matches)) => artifacts(db_connection_config, matches),
        Some(("envvars", matches)) => envvars(db_connection_config, matches),
        Some(("images", matches)) => images(db_connection_config, matches),
        Some(("submit", matches)) => submit(db_connection_config, matches),
        Some(("submits", matches)) => submits(db_connection_config, matches),
        Some(("jobs", matches)) => jobs(db_connection_config, matches),
        Some(("job", matches)) => job(db_connection_config, config, matches),
        Some(("log-of", matches)) => log_of(db_connection_config, matches),
        Some(("releases", matches)) => releases(db_connection_config, config, matches),
        Some((other, _)) => Err(anyhow!("Unknown subcommand: {}", other)),
        None => Err(anyhow!("No subcommand")),
    }
}

/// Implementation of the "db cli" subcommand
fn cli(db_connection_config: DbConnectionConfig<'_>, matches: &ArgMatches) -> Result<()> {
    trait PgCliCommand {
        fn run_for_uri(&self, dbcc: DbConnectionConfig<'_>) -> Result<()>;
    }

    struct Psql(PathBuf);
    impl PgCliCommand for Psql {
        fn run_for_uri(&self, dbcc: DbConnectionConfig<'_>) -> Result<()> {
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
                        info!("psql exited successfully");
                        Ok(())
                    } else {
                        Err(anyhow!("psql did not exit successfully"))
                            .with_context(|| match String::from_utf8(out.stderr) {
                                Ok(log) => anyhow!("{}", log),
                                Err(e) => anyhow!("Cannot parse log into valid UTF-8: {}", e),
                            })
                            .map_err(Error::from)
                    }
                })
        }
    }

    struct PgCli(PathBuf);
    impl PgCliCommand for PgCli {
        fn run_for_uri(&self, dbcc: DbConnectionConfig<'_>) -> Result<()> {
            Command::new(&self.0)
                .arg("--host")
                .arg(dbcc.database_host())
                .arg("--port")
                .arg(dbcc.database_port().to_string())
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
                        Err(anyhow!("pgcli did not exit successfully"))
                            .with_context(|| match String::from_utf8(out.stderr) {
                                Ok(log) => anyhow!("{}", log),
                                Err(e) => anyhow!("Cannot parse log into valid UTF-8: {}", e),
                            })
                            .map_err(Error::from)
                    }
                })
        }
    }

    matches
        .value_of("tool")
        .map(|s| vec![s])
        .unwrap_or_else(|| vec!["psql", "pgcli"])
        .into_iter()
        .filter_map(|s| which::which(s).ok().map(|path| (path, s)))
        .map(|(path, s)| match s {
            "psql" => Ok(Box::new(Psql(path)) as Box<dyn PgCliCommand>),
            "pgcli" => Ok(Box::new(PgCli(path)) as Box<dyn PgCliCommand>),
            prog => Err(anyhow!("Unsupported pg CLI program: {}", prog)),
        })
        .next()
        .transpose()?
        .ok_or_else(|| anyhow!("No Program found"))?
        .run_for_uri(db_connection_config)
}

fn setup(conn_cfg: DbConnectionConfig<'_>) -> Result<()> {
    let conn = conn_cfg.establish_connection()?;
    embedded_migrations::run_with_output(&conn, &mut std::io::stdout()).map_err(Error::from)
}

/// Implementation of the "db artifacts" subcommand
fn artifacts(conn_cfg: DbConnectionConfig<'_>, matches: &ArgMatches) -> Result<()> {
    use crate::schema::artifacts::dsl;

    let csv = matches.is_present("csv");
    let hdrs = crate::commands::util::mk_header(vec!["Path", "Released", "Job"]);
    let conn = conn_cfg.establish_connection()?;
    let data = matches
        .value_of("job_uuid")
        .map(uuid::Uuid::parse_str)
        .transpose()?
        .map(|job_uuid| -> Result<_> {
            dsl::artifacts
                .inner_join(schema::jobs::table)
                .left_join(schema::releases::table)
                .filter(schema::jobs::dsl::uuid.eq(job_uuid))
                .load::<(models::Artifact, models::Job, Option<models::Release>)>(&conn)
                .map_err(Error::from)
        })
        .unwrap_or_else(|| {
            dsl::artifacts
                .inner_join(schema::jobs::table)
                .left_join(schema::releases::table)
                .order_by(schema::artifacts::id.asc())
                .load::<(models::Artifact, models::Job, Option<models::Release>)>(&conn)
                .map_err(Error::from)
        })?
        .into_iter()
        .map(|(artifact, job, rel)| {
            let rel = rel
                .map(|r| r.release_date.to_string())
                .unwrap_or_else(|| String::from("no"));
            vec![
                artifact.path,
                rel,
                job.uuid.to_string(),
            ]
        })
        .collect::<Vec<_>>();

    if data.is_empty() {
        info!("No artifacts in database");
    } else {
        crate::commands::util::display_data(hdrs, data, csv)?;
    }

    Ok(())
}

/// Implementation of the "db envvars" subcommand
fn envvars(conn_cfg: DbConnectionConfig<'_>, matches: &ArgMatches) -> Result<()> {
    use crate::schema::envvars::dsl;

    let csv = matches.is_present("csv");
    let hdrs = crate::commands::util::mk_header(vec!["Name", "Value"]);
    let conn = conn_cfg.establish_connection()?;
    let data = dsl::envvars
        .load::<models::EnvVar>(&conn)?
        .into_iter()
        .map(|evar| vec![evar.name, evar.value])
        .collect::<Vec<_>>();

    if data.is_empty() {
        info!("No environment variables in database");
    } else {
        crate::commands::util::display_data(hdrs, data, csv)?;
    }

    Ok(())
}

/// Implementation of the "db images" subcommand
fn images(conn_cfg: DbConnectionConfig<'_>, matches: &ArgMatches) -> Result<()> {
    use crate::schema::images::dsl;

    let csv = matches.is_present("csv");
    let hdrs = crate::commands::util::mk_header(vec!["Name"]);
    let conn = conn_cfg.establish_connection()?;
    let data = dsl::images
        .load::<models::Image>(&conn)?
        .into_iter()
        .map(|image| vec![image.name])
        .collect::<Vec<_>>();

    if data.is_empty() {
        info!("No images in database");
    } else {
        crate::commands::util::display_data(hdrs, data, csv)?;
    }

    Ok(())
}

/// Implementation of the "db submit" subcommand
fn submit(conn_cfg: DbConnectionConfig<'_>, matches: &ArgMatches) -> Result<()> {
    let conn = conn_cfg.establish_connection()?;
    let submit_id = matches.value_of("submit")
        .map(uuid::Uuid::from_str)
        .transpose()
        .context("Parsing submit UUID")?
        .unwrap(); // safe by clap

    let submit = models::Submit::with_id(&conn, &submit_id)
        .with_context(|| anyhow!("Loading submit '{}' from DB", submit_id))?;

    let githash = models::GitHash::with_id(&conn, submit.repo_hash_id)
        .with_context(|| anyhow!("Loading GitHash '{}' from DB", submit.repo_hash_id))?;

    let jobs = schema::submits::table
        .inner_join(schema::jobs::table)
        .filter(schema::submits::uuid.eq(&submit_id))
        .select(schema::jobs::all_columns)
        .load::<models::Job>(&conn)
        .with_context(|| anyhow!("Loading jobs for submit = {}", submit_id))?;

    let n_jobs = jobs.len();
    let (jobs_unknown, jobs_success, jobs_err) = {
        let mut unkn = 0;
        let mut succ = 0;
        let mut err = 0;

        for j in jobs.iter() {
            match crate::log::ParsedLog::from_str(&j.log_text)?.is_successfull() {
                JobResult::Unknown => unkn += 1,
                JobResult::Success => succ += 1,
                JobResult::Errored => err += 1,
            }
        }

        (unkn, succ, err)
    };

    let out = std::io::stdout();
    let mut outlock = out.lock();

    indoc::writedoc!(outlock, r#"
            Submit   {submit_id}
            Date:    {submit_dt}
            Commit:  {submit_commit}
            Jobs:    {n_jobs}
            Success: {n_jobs_success}
            Unknown: {n_jobs_unknown}
            Errored: {n_jobs_err}

        "#,
        submit_id = submit.uuid.to_string().cyan(),
        submit_dt = submit.submit_time.to_string().cyan(),
        submit_commit = githash.hash.cyan(),
        n_jobs = n_jobs.to_string().cyan(),
        n_jobs_success = jobs_success.to_string().green(),
        n_jobs_unknown = jobs_unknown.to_string().red(),
        n_jobs_err = jobs_err.to_string().red(),
    )?;

    let header = crate::commands::util::mk_header(["Job", "Success", "Package", "Version", "Container", "Endpoint", "Image"].to_vec());
    let data = jobs.iter()
        .map(|job| {
            let image = models::Image::fetch_for_job(&conn, job)?
                .ok_or_else(|| anyhow!("Image for job {} not found", job.uuid))?;
            let package = models::Package::fetch_for_job(&conn, job)?
                .ok_or_else(|| anyhow!("Package for job {} not found", job.uuid))?;
            let endpoint = models::Endpoint::fetch_for_job(&conn, job)?
                .ok_or_else(|| anyhow!("Endpoint for job {} not found", job.uuid))?;

            Ok(vec![
                job.uuid.to_string().cyan(),
                match is_job_successfull(job)? {
                    Some(true) => "Success".green(),
                    Some(false) => "Error".red(),
                    None => "Unknown".yellow(),
                },
                package.name.cyan(),
                package.version.cyan(),
                job.container_hash.normal(),
                endpoint.name.normal(),
                image.name.normal(),
            ])
        })
        .collect::<Result<Vec<Vec<colored::ColoredString>>>>()?;
    crate::commands::util::display_data(header, data, false)
}

/// Implementation of the "db submits" subcommand
fn submits(conn_cfg: DbConnectionConfig<'_>, matches: &ArgMatches) -> Result<()> {
    let csv = matches.is_present("csv");
    let limit = matches.value_of("limit").map(i64::from_str).transpose()?;
    let hdrs = crate::commands::util::mk_header(vec!["Time", "UUID", "For Package", "For Package Version"]);
    let conn = conn_cfg.establish_connection()?;
    let commit = matches.value_of("for-commit");

    let query = schema::submits::table
        .order_by(schema::submits::id.desc()) // required for the --limit implementation
        .inner_join(schema::githashes::table.on(schema::submits::repo_hash_id.eq(schema::githashes::id)))
        .inner_join(schema::images::table)
        .into_boxed();

    let query = if let Some(commithash) = commit.as_ref() {
        query.filter(schema::githashes::hash.eq(commithash))
    } else {
        query
    };

    let query = if let Some(image) = matches.value_of("image") {
        query.filter(schema::images::name.eq(image))
    } else {
        query
    };

    let submits = if let Some(pkgname) = matches.value_of("with_pkg").map(String::from) {
        // In the case of a with_pkg command, we must execute two queries on the database, as the
        // diesel framework does not yet support aliases for queries (see
        // https://github.com/diesel-rs/diesel/pull/2254).
        // This is due to the fact that we need to join the packages table twice, once to filter
        // out all submits that did not include the "with pkg" and once to join the requested
        // package for the output.

        // Get all submits which included the package, but were not necessarily made _for_ the package
        let query = query
            .inner_join(schema::jobs::table)
            .inner_join(schema::packages::table.on(schema::jobs::package_id.eq(schema::packages::id)))
            .filter(schema::packages::name.eq(&pkgname));

        let query = if let Some(limit) = limit {
            query.limit(limit)
        } else {
            query
        };

        // Only load the IDs of the submits, so we can later use them to filter the submits
        let submit_ids = query.select(schema::submits::id).load::<i32>(&conn)?;

        schema::submits::table
            .order_by(schema::submits::id.desc()) // required for the --limit implementation
            .inner_join({
                schema::packages::table.on(schema::submits::requested_package_id.eq(schema::packages::id))
            })
            .filter(schema::submits::id.eq_any(submit_ids))
            .select((schema::submits::all_columns, schema::packages::all_columns))
            .load::<(models::Submit, models::Package)>(&conn)?
    } else if let Some(pkgname) = matches.value_of("for_pkg") {
        // Get all submits _for_ the package
        let query = query
            .inner_join({
                schema::packages::table.on(schema::submits::requested_package_id.eq(schema::packages::id))
            })
            .filter(schema::packages::dsl::name.eq(&pkgname));

        if let Some(limit) = limit {
            query.limit(limit)
        } else {
            query
        }
        .select((schema::submits::all_columns, schema::packages::all_columns))
        .load::<(models::Submit, models::Package)>(&conn)?
    } else if let Some(limit) = limit {
        query
            .inner_join({
                schema::packages::table.on(schema::submits::requested_package_id.eq(schema::packages::id))
            })
            .select((schema::submits::all_columns, schema::packages::all_columns))
            .limit(limit)
            .load::<(models::Submit, models::Package)>(&conn)?
    } else {
        query.inner_join({
                schema::packages::table.on(schema::submits::requested_package_id.eq(schema::packages::id))
            })
            .select((schema::submits::all_columns, schema::packages::all_columns))
            .load::<(models::Submit, models::Package)>(&conn)?
    };

    // Helper to map (Submit, Package) -> Vec<String>
    let submit_to_vec = |(submit, package): (models::Submit, models::Package)| {
        vec![
            submit.submit_time.to_string(),
            submit.uuid.to_string(),
            package.name,
            package.version,
        ]
    };

    let data = submits.into_iter().rev().map(submit_to_vec).collect::<Vec<_>>();

    if data.is_empty() {
        info!("No submits in database");
    } else {
        crate::commands::util::display_data(hdrs, data, csv)?;
    }

    Ok(())
}

/// Implementation of the "db jobs" subcommand
fn jobs(conn_cfg: DbConnectionConfig<'_>, matches: &ArgMatches) -> Result<()> {
    let csv = matches.is_present("csv");
    let hdrs = crate::commands::util::mk_header(vec![
        "Submit",
        "Job",
        "Time",
        "Endpoint",
        "Success",
        "Package",
        "Version",
    ]);
    let conn = conn_cfg.establish_connection()?;
    let older_than_filter = get_date_filter("older_than", matches)?;
    let newer_than_filter = get_date_filter("newer_than", matches)?;

    let mut sel = schema::jobs::table
        .inner_join(schema::submits::table)
        .inner_join(schema::endpoints::table)
        .inner_join(schema::packages::table)
        .into_boxed();

    if let Some(submit_uuid) = matches.value_of("submit_uuid").map(uuid::Uuid::parse_str).transpose()? {
        sel = sel.filter(schema::submits::uuid.eq(submit_uuid))
    }

    // Filter for environment variables from the CLI
    //
    // If we get a filter for environment on CLI, we fetch all job ids that are associated with the
    // passed environment variables and make `sel` filter for those.
    if let Some((name, val)) = matches.value_of("env_filter").map(crate::util::env::parse_to_env).transpose()? {
        debug!("Filtering for ENV: {} = {}", name, val);
        let jids = schema::envvars::table
            .filter({
                use crate::diesel::BoolExpressionMethods;
                schema::envvars::dsl::name.eq(name.as_ref())
                    .and(schema::envvars::dsl::value.eq(val))
            })
            .inner_join(schema::job_envs::table)
            .select(schema::job_envs::job_id)
            .load::<i32>(&conn)?;

        debug!("Filtering for these IDs (because of env filter): {:?}", jids);
        sel = sel.filter(schema::jobs::dsl::id.eq_any(jids));
    }

    if let Some(datetime) = older_than_filter.as_ref() {
        sel = sel.filter(schema::submits::dsl::submit_time.lt(datetime))
    }

    if let Some(datetime) = newer_than_filter.as_ref() {
        sel = sel.filter(schema::submits::dsl::submit_time.gt(datetime))
    }

    if let Some(limit) = matches.value_of("limit").map(i64::from_str).transpose()? {
        sel = sel.limit(limit)
    }

    if let Some(ep_name) = matches.value_of("endpoint") {
        sel = sel.filter(schema::endpoints::name.eq(ep_name))
    }

    if let Some(pkg_name) = matches.value_of("package") {
        sel = sel.filter(schema::packages::name.eq(pkg_name))
    }

    let data = sel
        .order_by(schema::jobs::id.desc()) // required for the --limit implementation
        .load::<(models::Job, models::Submit, models::Endpoint, models::Package)>(&conn)?
        .into_iter()
        .rev() // required for the --limit implementation
        .map(|(job, submit, ep, package)| {
            let success = is_job_successfull(&job)?
                .map(|b| if b { "yes" } else { "no" })
                .map(String::from)
                .unwrap_or_else(|| String::from("unknown"));

            Ok(vec![
                submit.uuid.to_string(),
                job.uuid.to_string(),
                submit.submit_time.format("%Y-%m-%d %H:%M:%S").to_string(),
                ep.name,
                success,
                package.name,
                package.version,
            ])
        })
        .collect::<Result<Vec<_>>>()?;

    if data.is_empty() {
        info!("No submits in database");
    } else {
        crate::commands::util::display_data(hdrs, data, csv)?;
    }

    Ok(())
}

/// Implementation of the "db job" subcommand
fn job(conn_cfg: DbConnectionConfig<'_>, config: &Configuration, matches: &ArgMatches) -> Result<()> {
    let script_highlight = !matches.is_present("no_script_highlight");
    let script_line_numbers = !matches.is_present("no_script_line_numbers");
    let configured_theme = config.script_highlight_theme();
    let show_log = matches.is_present("show_log");
    let show_script = matches.is_present("show_script");
    let csv = matches.is_present("csv");
    let conn = conn_cfg.establish_connection()?;
    let job_uuid = matches
        .value_of("job_uuid")
        .map(uuid::Uuid::parse_str)
        .transpose()?
        .unwrap();

    let data = schema::jobs::table
        .filter(schema::jobs::dsl::uuid.eq(job_uuid))
        .inner_join(schema::submits::table)
        .inner_join(schema::endpoints::table)
        .inner_join(schema::packages::table)
        .inner_join(schema::images::table)
        .first::<(
            models::Job,
            models::Submit,
            models::Endpoint,
            models::Package,
            models::Image,
        )>(&conn)?;

    trace!("Parsing log");
    let parsed_log = crate::log::ParsedLog::from_str(&data.0.log_text)?;
    trace!("Parsed log = {:?}", parsed_log);
    let success = parsed_log.is_successfull();
    trace!("log successfull = {:?}", success);

    if csv {
        let hdrs = crate::commands::util::mk_header(vec![
            "UUID",
            "Success",
            "Package Name",
            "Package Version",
            "Ran on",
            "Image Name",
            "Container",
        ]);

        let data = vec![vec![
            data.0.uuid.to_string(),
            String::from(match success {
                JobResult::Success => "yes",
                JobResult::Errored => "no",
                JobResult::Unknown => "unknown",
            }),
            data.3.name.to_string(),
            data.3.version.to_string(),
            data.2.name.to_string(),
            data.4.name.to_string(),
            data.0.container_hash,
        ]];
        crate::commands::util::display_data(hdrs, data, csv)
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
        let s = indoc::formatdoc!(
            r#"
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
                JobResult::Success => data.0.uuid.to_string().green(),
                JobResult::Errored => data.0.uuid.to_string().red(),
                JobResult::Unknown => data.0.uuid.to_string().cyan(),
            },
            submit_uuid = data.1.uuid.to_string().cyan(),
            succeeded = match success {
                JobResult::Success => String::from("yes").green(),
                JobResult::Errored => String::from("no").red(),
                JobResult::Unknown => String::from("unknown").cyan(),
            },
            package_name = data.3.name.cyan(),
            package_version = data.3.version.cyan(),
            endpoint_name = data.2.name.cyan(),
            image_name = data.4.name.cyan(),
            container_hash = data.0.container_hash.cyan(),
            script_len = format!("{:<4}", data.0.script_text.lines().count()).cyan(),
            log_len = format!("{:<4}", data.0.log_text.lines().count()).cyan(),
        );
        writeln!(out, "{}", s)?;

        if let Some(envs) = env_vars {
            let s = indoc::formatdoc!(
                r#"
                ---

                {envs}

            "#,
                envs = envs
            );
            writeln!(out, "{}", s)?;
        }

        if show_script {
            let theme = configured_theme.as_ref().ok_or_else(|| {
                anyhow!("Highlighting for script enabled, but no theme configured")
            })?;
            let script = Script::from(data.0.script_text);
            let script = crate::ui::script_to_printable(
                &script,
                script_highlight,
                theme,
                script_line_numbers,
            )?;

            let s = indoc::formatdoc!(
                r#"
                ---

                {script}

            "#,
                script = script
            );
            writeln!(out, "{}", s)?;
        }

        if show_log {
            let log = parsed_log
                .into_iter()
                .map(|line_item| line_item.display().map(|d| d.to_string()))
                .collect::<Result<Vec<_>>>()?
                .into_iter() // ugly, but hey... not important right now.
                .join("\n");

            let s = indoc::formatdoc!(
                r#"
                ---

                {log}

            "#,
                log = log
            );
            writeln!(out, "{}", s)?;
        }

        Ok(())
    }
}

/// Implementation of the subcommand "db log-of"
fn log_of(conn_cfg: DbConnectionConfig<'_>, matches: &ArgMatches) -> Result<()> {
    let conn   = conn_cfg.establish_connection()?;
    let job_uuid = matches
        .value_of("job_uuid")
        .map(uuid::Uuid::parse_str)
        .transpose()?
        .unwrap();
    let out = std::io::stdout();
    let mut lock = out.lock();

    schema::jobs::table
        .filter(schema::jobs::dsl::uuid.eq(job_uuid))
        .select(schema::jobs::dsl::log_text)
        .first::<String>(&conn)
        .map_err(Error::from)
        .and_then(|s| crate::log::ParsedLog::from_str(&s))?
        .into_iter()
        .map(|line| line.display().and_then(|d| writeln!(lock, "{}", d).map_err(Error::from)))
        .collect::<Result<Vec<()>>>()
        .map(|_| ())
}

/// Implementation of the "db releases" subcommand
fn releases(conn_cfg: DbConnectionConfig<'_>, config: &Configuration, matches: &ArgMatches) -> Result<()> {
    let csv    = matches.is_present("csv");
    let conn   = conn_cfg.establish_connection()?;
    let header = crate::commands::util::mk_header(["Package", "Version", "Date", "Path"].to_vec());
    let mut query = schema::jobs::table
        .inner_join(schema::packages::table)
        .inner_join(schema::artifacts::table)
        .inner_join(schema::releases::table
            .on(schema::releases::artifact_id.eq(schema::artifacts::id)))
        .inner_join(schema::release_stores::table
            .on(schema::release_stores::id.eq(schema::releases::release_store_id)))
        .order_by(schema::packages::dsl::name.asc())
        .then_order_by(schema::packages::dsl::version.asc())
        .then_order_by(schema::releases::release_date.asc())
        .into_boxed();

    if let Some(date) = crate::commands::util::get_date_filter("older_than", matches)? {
        query = query.filter(schema::releases::release_date.lt(date));
    }

    if let Some(date) = crate::commands::util::get_date_filter("newer_than", matches)? {
        query = query.filter(schema::releases::release_date.gt(date));
    }

    if let Some(store) = matches.value_of("store") {
        query = query.filter(schema::release_stores::dsl::store_name.eq(store));
    }

    if let Some(pkg) = matches.value_of("package") {
        query = query.filter(schema::packages::dsl::name.eq(pkg));
    }

    let data = query
        .select({
            let art = schema::artifacts::all_columns;
            let pac = schema::packages::all_columns;
            let rel = schema::releases::all_columns;
            let rst = schema::release_stores::all_columns;
            (art, pac, rel, rst)
        })
        .load::<(models::Artifact, models::Package, models::Release, models::ReleaseStore)>(&conn)?
        .into_iter()
        .filter_map(|(art, pack, rel, rstore)| {
            let p = config.releases_directory().join(rstore.store_name).join(&art.path);

            if p.is_file() {
                Some(vec![
                    pack.name,
                    pack.version,
                    rel.release_date.to_string(),
                    p.display().to_string(),
                ])
            } else {
                log::warn!("Released file for {} {} not found: {}", pack.name, pack.version, p.display());
                None
            }
        })
        .collect::<Vec<Vec<_>>>();

    crate::commands::util::display_data(header, data, csv)
}

/// Check if a job is successful
///
/// Returns Ok(None) if cannot be decided
fn is_job_successfull(job: &models::Job) -> Result<Option<bool>> {
    crate::log::ParsedLog::from_str(&job.log_text).map(|pl| pl.is_successfull().to_bool())
}

