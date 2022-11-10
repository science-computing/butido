//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'versions_of' subcommand

use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use log::trace;

use crate::package::PackageName;
use crate::repository::Repository;

/// Implementation of the "versions_of" subcommand
pub async fn versions_of(matches: &ArgMatches, repo: Repository) -> Result<()> {
    use filters::filter::Filter;
    use std::io::Write;

    let package_filter = {
        let name = matches
            .get_one::<String>("package_name")
            .map(String::clone)
            .map(PackageName::from)
            .unwrap();
        trace!("Checking for package with name = {}", name);

        crate::util::filters::build_package_filter_by_name(name)
    };

    let mut stdout = std::io::stdout();
    repo.packages()
        .filter(|package| package_filter.filter(package))
        .inspect(|pkg| trace!("Found package: {:?}", pkg))
        .map(|pkg| writeln!(stdout, "{}", pkg.version()).map_err(Error::from))
        .collect::<Result<Vec<_>>>()
        .map(|_| ())
}
