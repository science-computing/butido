//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'what_depends' subcommand

use std::io::Write;

use anyhow::Result;
use clap::ArgMatches;
use futures::stream::StreamExt;
use futures::stream::TryStreamExt;
use log::trace;
use resiter::Filter;
use resiter::Map;

use crate::commands::util::getbool;
use crate::config::*;
use crate::package::PackageName;
use crate::repository::Repository;
use crate::ui::*;

/// Implementation of the "what_depends" subcommand
pub async fn what_depends(
    matches: &ArgMatches,
    config: &Configuration,
    repo: Repository,
) -> Result<()> {
    use filters::failable::filter::FailableFilter;

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

    let package_filter = {
        let name = matches
            .value_of("package_name")
            .map(String::from)
            .map(PackageName::from)
            .unwrap();

        crate::util::filters::build_package_filter_by_dependency_name(
            &name,
            print_build_deps,
            print_runtime_deps,
        )
    };

    let hb = crate::ui::handlebars_for_package_printing(config.package_print_format())?;
    let stdout = std::io::stdout();
    let mut outlock = stdout.lock();

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

    let mut i = 0;
    let iter = repo
        .packages()
        .map(|package| package_filter.filter(package).map(|b| (b, package)))
        .filter_ok(|(b, _)| *b)
        .map_ok(|tpl| tpl.1)
        .inspect(|pkg| trace!("Found package: {:?}", pkg))
        .map_ok(|p| { // poor mans enumerate_ok()
            i += 1;
            p.prepare_print(config, &flags, &hb, i)
        });

    tokio_stream::iter(iter)
        .map(|pp| pp.and_then(|p| p.into_displayable()))
        .try_for_each(|p| {
            let r = writeln!(&mut outlock, "{}", p).map_err(anyhow::Error::from);
            futures::future::ready(r)
        })
        .await
}
