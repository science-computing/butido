//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'lint' subcommand

use std::convert::TryFrom;
use std::path::Path;

use anyhow::anyhow;
use anyhow::Result;
use clap::ArgMatches;

use crate::config::*;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;
use crate::util::progress::ProgressBars;

/// Implementation of the "lint" subcommand
pub async fn lint(
    repo_path: &Path,
    matches: &ArgMatches,
    progressbars: ProgressBars,
    config: &Configuration,
    repo: Repository,
) -> Result<()> {
    let linter = crate::ui::find_linter_command(repo_path, config)?
        .ok_or_else(|| anyhow!("No linter command found"))?;
    let pname = matches
        .get_one::<String>("package_name")
        .map(|s| s.to_owned())
        .map(PackageName::from);
    let pvers = matches
        .get_one::<String>("package_version")
        .map(|s| s.to_owned())
        .map(PackageVersionConstraint::try_from)
        .transpose()?;

    let bar = progressbars.bar()?;
    bar.set_message("Linting package scripts...");

    let iter = repo
        .packages()
        .filter(|p| pname.as_ref().map(|n| p.name() == n).unwrap_or(true))
        .filter(|p| {
            pvers
                .as_ref()
                .map(|v| v.matches(p.version()))
                .unwrap_or(true)
        });

    crate::commands::util::lint_packages(iter, &linter, config, bar).await
}
