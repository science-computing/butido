//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use resiter::AndThen;

use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;
use crate::package::Tree;
use crate::util::progress::ProgressBars;

pub async fn tree_of(matches: &ArgMatches, repo: Repository, progressbars: ProgressBars) -> Result<()> {
    let pname = matches.value_of("package_name").map(String::from).map(PackageName::from);
    let pvers = matches.value_of("package_version").map(String::from).map(PackageVersionConstraint::new).transpose()?;

    repo.packages()
        .filter(|p| pname.as_ref().map(|n| p.name() == n).unwrap_or(true))
        .filter(|p| pvers.as_ref().map(|v| v.matches(p.version())).unwrap_or(true))
        .map(|package| {
            let bar_tree_building = progressbars.bar();
            let mut tree = Tree::new();
            let _ = tree.add_package(package.clone(), &repo, bar_tree_building.clone())?;
            bar_tree_building.finish_with_message("Finished loading Tree");
            Ok(tree)
        })
        .and_then_ok(|tree| {
            let stdout = std::io::stdout();
            let mut outlock = stdout.lock();

            tree.display().iter().try_for_each(|d| ptree::write_tree(d, &mut outlock).map_err(Error::from))
        })
        .collect::<Result<()>>()
}


