//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Utility module for subcommand implementation helpers

use std::fmt::Display;
use std::io::Write;
use std::path::Path;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use itertools::Itertools;
use regex::Regex;
use tokio_stream::StreamExt;
use tracing::{error, info, trace};

use crate::config::*;
use crate::package::Package;
use crate::package::PhaseName;
use crate::package::ScriptBuilder;
use crate::package::Shebang;

/// Helper for getting a boolean value by name form the argument object
pub fn getbool(m: &ArgMatches, name: &str, cmp: &str) -> bool {
    // unwrap is safe here because clap is configured with default values
    m.get_many::<String>(name).unwrap().any(|v| v == cmp)
}

/// Helper function to lint all packages in an interator
pub async fn lint_packages<'a, I>(
    iter: I,
    linter: &Path,
    config: &Configuration,
    bar: indicatif::ProgressBar,
) -> Result<()>
where
    I: Iterator<Item = &'a Package> + 'a,
{
    let shebang = Shebang::from(config.shebang().clone());
    bar.set_length({
        let (lower, upper) = iter.size_hint();
        upper.unwrap_or(lower) as u64
    });

    let lint_results = iter
        .map(|pkg| {
            let shebang = shebang.clone();
            let bar = bar.clone();
            async move {
                trace!("Linting script of {} {} with '{}'", pkg.name(), pkg.version(), linter.display());
                all_phases_available(pkg, config.available_phases())?;

                let cmd = tokio::process::Command::new(linter);
                let script = ScriptBuilder::new(&shebang)
                    .build(pkg, config.available_phases(), *config.strict_script_interpolation())?;

                let (status, stdout, stderr) = script.lint(cmd).await?;
                bar.inc(1);
                Ok((pkg.name().clone(), pkg.version().clone(), status, stdout, stderr))
            }
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<Vec<_>>>()
        .await?
        .into_iter()
        .map(|tpl| {
            let pkg_name = tpl.0;
            let pkg_vers = tpl.1;
            let status = tpl.2;
            let stdout = tpl.3;
            let stderr = tpl.4;

            if status.success() {
                info!("Linting {pkg_name} {pkg_vers} script ({status}):\nstdout:\n{stdout}\n\nstderr:\n\n{stderr}",
                    pkg_name = pkg_name,
                    pkg_vers = pkg_vers,
                    status = status,
                    stdout = stdout,
                    stderr = stderr
                );
                true
            } else {
                error!("Linting {pkg_name} {pkg_vers} errored ({status}):\n\nstdout:\n{stdout}\n\nstderr:\n{stderr}\n\n",
                    pkg_name = pkg_name,
                    pkg_vers = pkg_vers,
                    status = status,
                    stdout = stdout,
                    stderr = stderr
                );
                false
            }
        })
        .collect::<Vec<_>>();

    let lint_ok = lint_results.iter().all(|b| *b);

    if !lint_ok {
        bar.finish_with_message("Linting errored");
        Err(anyhow!("Linting was not successful"))
    } else {
        bar.finish_with_message(format!(
            "Finished linting {} package scripts",
            lint_results.len()
        ));
        Ok(())
    }
}

/// Check whether all phases are available in the package,
/// generate a nice error message if one is not.
fn all_phases_available(pkg: &Package, available_phases: &[PhaseName]) -> Result<()> {
    let package_phasenames = pkg.phases().keys().collect::<Vec<_>>();

    if let Some(phase) = package_phasenames
        .iter()
        .find(|name| !available_phases.contains(name))
    {
        return Err(anyhow!(
            "Phase '{}' available in {} {}, but not in config",
            phase.as_str(),
            pkg.name(),
            pkg.version()
        ));
    }

    if let Some(phase) = available_phases
        .iter()
        .find(|name| !package_phasenames.contains(name))
    {
        return Err(anyhow!(
            "Phase '{}' not configured in {} {}",
            phase.as_str(),
            pkg.name(),
            pkg.version()
        ));
    }

    Ok(())
}

/// Helper function to make a package name regex out of a String
pub fn mk_package_name_regex(regex: &str) -> Result<Regex> {
    let mut builder = regex::RegexBuilder::new(regex);

    #[allow(clippy::identity_op)]
    builder.size_limit(1 * 1024 * 1024); // max size for the regex is 1MB. Should be enough for everyone

    builder
        .build()
        .with_context(|| anyhow!("Failed to build regex from '{}'", regex))
        .map_err(Error::from)
}

/// Make a header column for the ascii_table crate
pub fn mk_header(vec: Vec<&str>) -> Vec<ascii_table::Column> {
    vec.into_iter()
        .map(|name| {
            let mut column = ascii_table::Column::default();
            column.set_header::<String>(name.into());
            column.set_align(ascii_table::Align::Left);
            column
        })
        .collect()
}

/// Display the passed data as nice ascii table,
/// or, if stdout is a pipe, print it nicely parseable
///
/// If `csv` is `true`, convert the data to CSV and print that instead.
pub fn display_data<D: Display>(
    headers: Vec<ascii_table::Column>,
    data: Vec<Vec<D>>,
    csv: bool,
) -> Result<()> {
    if data.is_empty() {
        return Ok(());
    }

    if csv {
        use csv::WriterBuilder;
        let mut wtr = WriterBuilder::new().from_writer(vec![]);
        for record in data.into_iter() {
            let r: Vec<String> = record.into_iter().map(|e| e.to_string()).collect();

            wtr.write_record(&r)?;
        }

        let out = std::io::stdout();
        let mut lock = out.lock();

        wtr.into_inner()
            .map_err(Error::from)
            .and_then(|t| String::from_utf8(t).map_err(Error::from))
            .and_then(|text| writeln!(lock, "{text}").map_err(Error::from))
    } else if atty::is(atty::Stream::Stdout) {
        let mut ascii_table = ascii_table::AsciiTable::default();
        ascii_table.set_max_width(
            terminal_size::terminal_size()
                .map(|tpl| tpl.0 .0 as usize) // an ugly interface indeed!
                .unwrap_or(80),
        );

        headers.into_iter().enumerate().for_each(|(i, c)| {
            *ascii_table.column(i) = c;
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

pub fn get_date_filter(
    name: &str,
    matches: &ArgMatches,
) -> Result<Option<chrono::DateTime<chrono::Local>>> {
    matches
        .get_one::<String>(name)
        .map(|s| {
            trace!("Parsing duration: '{}'", s);
            humantime::parse_duration(s)
                .map_err(Error::from)
                .or_else(|_| {
                    trace!("Parsing time: '{}'", s);
                    humantime::parse_rfc3339_weak(s)
                        .map_err(Error::from)
                        .and_then(|d| d.elapsed().map_err(Error::from))
                })
                .or_else(|_| {
                    let s = format!("{s} 00:00:00");
                    trace!("Parsing time: '{}'", s);
                    humantime::parse_rfc3339_weak(&s)
                        .map_err(Error::from)
                        .and_then(|d| d.elapsed().map_err(Error::from))
                })
        })
        .transpose()?
        .map(chrono::Duration::from_std)
        .transpose()?
        .map(|dur| {
            chrono::offset::Local::now()
                .checked_sub_signed(dur)
                .ok_or_else(|| anyhow!("Time calculation would overflow"))
                .with_context(|| anyhow!("Cannot subtract {} from 'now'", dur))
                .map_err(Error::from)
        })
        .transpose()
}
