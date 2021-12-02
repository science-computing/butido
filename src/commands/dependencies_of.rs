//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'dependencies-of' subcommand

use std::io::Write;

use anyhow::Result;
use clap::ArgMatches;
use futures::stream::StreamExt;
use futures::stream::TryStreamExt;
use log::trace;

use crate::commands::util::getbool;
use crate::config::*;
use crate::package::PackageName;
use crate::repository::Repository;
use crate::ui::*;

/// Implementation of the "dependencies_of" subcommand
pub async fn dependencies_of(
    matches: &ArgMatches,
    config: &Configuration,
    repo: Repository,
) -> Result<()> {
    use filters::filter::Filter;

    let package_filter = {
        let name = matches
            .value_of("package_name")
            .map(String::from)
            .map(PackageName::from)
            .unwrap();
        trace!("Checking for package with name = {}", name);

        crate::util::filters::build_package_filter_by_name(name)
    };

    let format = config.package_print_format();
    let hb = crate::ui::handlebars_for_package_printing(format)?;
    let stdout = std::io::stdout();
    let mut outlock = stdout.lock();

    let print_runtime_deps = getbool(
        matches,
        "dependency_type",
        crate::cli::IDENT_DEPENDENCY_TYPE_RUNTIME,
    );
    let print_build_deps = getbool(
        matches,
        "dependency_type",
        crate::cli::IDENT_DEPENDENCY_TYPE_BUILD,
    );

    trace!(
        "Printing packages with format = '{}', runtime: {}, build: {}",
        format,
        print_runtime_deps,
        print_build_deps
    );

    let flags = crate::ui::PackagePrintFlags {
        print_all: false,
        print_runtime_deps,
        print_build_deps,
        print_sources: false,
        print_dependencies: true,
        print_patches: false,
        print_env: false,
        print_flags: false,
        print_allowed_images: false,
        print_denied_images: false,
        print_phases: false,
        print_script: false,
        script_line_numbers: false,
        script_highlighting: false,
    };

    let iter = repo
        .packages()
        .filter(|package| package_filter.filter(package))
        .inspect(|pkg| trace!("Found package: {:?}", pkg))
        .enumerate()
        .map(|(i, p)| p.prepare_print(config, &flags, &hb, i));

    tokio_stream::iter(iter)
        .map(|pp| pp.into_displayable())
        .try_for_each(|p| {
            let r = writeln!(&mut outlock, "{}", p).map_err(anyhow::Error::from);
            futures::future::ready(r)
        })
        .await
}
