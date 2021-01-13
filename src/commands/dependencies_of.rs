//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::Result;
use clap::ArgMatches;
use log::trace;

use crate::commands::util::getbool;
use crate::config::*;
use crate::package::PackageName;
use crate::repository::Repository;

pub async fn dependencies_of(matches: &ArgMatches, config: &Configuration, repo: Repository) -> Result<()> {
    use filters::filter::Filter;

    let package_filter = {
        let name = matches.value_of("package_name").map(String::from).map(PackageName::from).unwrap();
        trace!("Checking for package with name = {}", name);

        crate::util::filters::build_package_filter_by_name(name)
    };

    let format = config.package_print_format();
    let mut stdout = std::io::stdout();
    let iter = repo.packages().filter(|package| package_filter.filter(package))
        .inspect(|pkg| trace!("Found package: {:?}", pkg));

    let print_runtime_deps     = getbool(matches, "dependency_type", crate::cli::IDENT_DEPENDENCY_TYPE_RUNTIME);
    let print_build_deps       = getbool(matches, "dependency_type", crate::cli::IDENT_DEPENDENCY_TYPE_BUILD);

    trace!("Printing packages with format = '{}', runtime: {}, build: {}",
           format,
           print_runtime_deps,
           print_build_deps);

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

    crate::ui::print_packages(&mut stdout, format, iter, config, &flags)
}

