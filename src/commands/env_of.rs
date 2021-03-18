//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::convert::TryFrom;

use anyhow::Result;
use clap::ArgMatches;
use log::trace;

use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;

/// Implementation of the "env_of" subcommand
pub async fn env_of(matches: &ArgMatches, repo: Repository) -> Result<()> {
    use filters::filter::Filter;
    use std::io::Write;

    let package_filter = {
        let name = matches
            .value_of("package_name")
            .map(String::from)
            .map(PackageName::from)
            .unwrap();
        let constraint = matches
            .value_of("package_version_constraint")
            .map(PackageVersionConstraint::try_from)
            .unwrap()?;
        trace!(
            "Checking for package with name = {} and version = {:?}",
            name,
            constraint
        );

        crate::util::filters::build_package_filter_by_name(name)
            .and(crate::util::filters::build_package_filter_by_version_constraint(constraint))
    };

    let mut stdout = std::io::stdout();
    repo.packages()
        .filter(|package| package_filter.filter(package))
        .inspect(|pkg| trace!("Found package: {:?}", pkg))
        .try_for_each(|pkg| {
            if let Some(hm) = pkg.environment() {
                for (key, value) in hm {
                    writeln!(stdout, "{} = '{}'", key, value)?;
                }
            } else {
                writeln!(stdout, "No environment")?;
            }

            Ok(())
        })
}
