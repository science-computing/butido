//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'find-pkg' subcommand

use std::convert::TryFrom;

use anyhow::Context;
use anyhow::Result;
use clap::ArgMatches;
use log::trace;
use futures::stream::StreamExt;
use futures::stream::TryStreamExt;

use crate::config::Configuration;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;
use crate::ui::*;

/// Implementation of the "find_pkg" subcommand
pub async fn find_pkg(
    matches: &ArgMatches,
    config: &Configuration,
    repo: Repository,
) -> Result<()> {
    use std::io::Write;

    let package_name_regex = crate::commands::util::mk_package_name_regex({
        matches.value_of("package_name_regex").unwrap() // safe by clap
    })?;

    let package_version_constraint = matches
        .value_of("package_version_constraint")
        .map(PackageVersionConstraint::try_from)
        .transpose()
        .context("Parsing package version constraint")
        .context("A valid package version constraint looks like this: '=1.0.0'")?;

    let iter = repo
        .packages()
        .filter(|p| package_name_regex.captures(p.name()).is_some())
        .filter(|p| {
            package_version_constraint
                .as_ref()
                .map(|v| v.matches(p.version()))
                .unwrap_or(true)
        })
        .inspect(|pkg| trace!("Found package: {:?}", pkg));

    let out = std::io::stdout();
    let mut outlock = out.lock();
    if matches.is_present("terse") {
        for p in iter {
            writeln!(outlock, "{} {}", p.name(), p.version())?;
        }
        Ok(())
    } else {
        let flags = crate::ui::PackagePrintFlags {
            print_all: matches.is_present("show_all"),
            print_runtime_deps: crate::commands::util::getbool(
                matches,
                "dependency_type",
                crate::cli::IDENT_DEPENDENCY_TYPE_RUNTIME,
            ),
            print_build_deps: crate::commands::util::getbool(
                matches,
                "dependency_type",
                crate::cli::IDENT_DEPENDENCY_TYPE_BUILD,
            ),
            print_sources: matches.is_present("show_sources"),
            print_dependencies: matches.is_present("show_dependencies"),
            print_patches: matches.is_present("show_patches"),
            print_env: matches.is_present("show_env"),
            print_flags: matches.is_present("show_flags"),
            print_allowed_images: matches.is_present("show_allowed_images"),
            print_denied_images: matches.is_present("show_denied_images"),
            print_phases: matches.is_present("show_phases"),
            print_script: matches.is_present("show_script"),
            script_line_numbers: !matches.is_present("no_script_line_numbers"),
            script_highlighting: !matches.is_present("no_script_highlight"),
        };

        let format = config.package_print_format();
        let hb = crate::ui::handlebars_for_package_printing(format)?;

        tokio_stream::iter({
            iter.enumerate()
                .map(|(i, p)| p.prepare_print(config, &flags, &hb, i))
        })
        .map(|pp| pp.into_displayable())
        .try_for_each(|p| {
            let r = writeln!(&mut outlock, "{}", p).map_err(anyhow::Error::from);
            futures::future::ready(r)
        })
        .await
    }
}
