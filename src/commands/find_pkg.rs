//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use log::trace;

use crate::config::Configuration;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;

pub async fn find_pkg(matches: &ArgMatches, config: &Configuration, repo: Repository) -> Result<()> {
    use std::io::Write;

    let package_name_regex = matches
        .value_of("package_name_regex")
        .map(regex::RegexBuilder::new)
        .map(|mut builder| {
            builder.size_limit(1 * 1024 * 1024); // max size for the regex is 1MB. Should be enough for everyone
            builder.build()
                .with_context(|| anyhow!("Failed to build regex from '{}'", matches.value_of("package_name_regex").unwrap()))
                .map_err(Error::from)
        })
        .unwrap()?; // safe by clap

    let package_version_constraint = matches
        .value_of("package_version_constraint")
        .map(String::from)
        .map(PackageVersionConstraint::new)
        .transpose()
        .context("Parsing package version constraint")
        .context("A valid package version constraint looks like this: '=1.0.0'")?;

    let iter = repo.packages()
        .filter(|p| package_name_regex.captures(p.name()).is_some())
        .filter(|p| package_version_constraint.as_ref().map(|v| v.matches(p.version())).unwrap_or(true))
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
            print_all           : matches.is_present("show_all"),
            print_runtime_deps  : crate::commands::util::getbool(matches, "dependency_type", crate::cli::IDENT_DEPENDENCY_TYPE_RUNTIME),
            print_build_deps    : crate::commands::util::getbool(matches, "dependency_type", crate::cli::IDENT_DEPENDENCY_TYPE_BUILD),
            print_sources       : matches.is_present("show_sources"),
            print_dependencies  : matches.is_present("show_dependencies"),
            print_patches       : matches.is_present("show_patches"),
            print_env           : matches.is_present("show_env"),
            print_flags         : matches.is_present("show_flags"),
            print_allowed_images: matches.is_present("show_allowed_images"),
            print_denied_images : matches.is_present("show_denied_images"),
            print_phases        : matches.is_present("show_phases"),
            print_script        : matches.is_present("show_script"),
            script_line_numbers : !matches.is_present("no_script_line_numbers"),
            script_highlighting : !matches.is_present("no_script_highlight"),
        };

        let format = config.package_print_format();
        crate::ui::print_packages(&mut outlock, format, iter, config, &flags)
    }
}



