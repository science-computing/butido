//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'env-of' subcommand

use anyhow::Result;
use clap::ArgMatches;
use tracing::trace;

use crate::package::PackageName;
use crate::package::PackageVersion;
use crate::repository::Repository;

/// Implementation of the "env_of" subcommand
pub async fn env_of(matches: &ArgMatches, repo: Repository) -> Result<()> {
    use filters::filter::Filter;
    use std::io::Write;

    let package_filter = {
        let name = matches
            .get_one::<String>("package_name")
            .map(|s| s.to_owned())
            .map(PackageName::from)
            .unwrap();
        let constraint = matches
            .get_one::<String>("package_version_constraint")
            .map(|s| s.to_owned())
            .map(PackageVersion::try_from)
            .unwrap()?;
        trace!(
            "Checking for package with name = {} and version = {:?}",
            name,
            constraint
        );

        crate::util::filters::build_package_filter_by_name(name).and(
            crate::util::filters::build_package_filter_by_version(constraint),
        )
    };

    let mut stdout = std::io::stdout();
    repo.packages()
        .filter(|package| package_filter.filter(package))
        .inspect(|pkg| trace!("Found package: {:?}", pkg))
        .try_for_each(|pkg| {
            if let Some(hm) = pkg.environment() {
                for (key, value) in hm {
                    writeln!(stdout, "{key} = '{value}'")?;
                }
            } else {
                writeln!(stdout, "No environment")?;
            }

            Ok(())
        })
}
