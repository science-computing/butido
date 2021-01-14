//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::io::Write;

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

    fn print_package_tree(out: &mut dyn Write, indent: usize, tree: Tree) -> Result<()> {
        for (pkg, tree) in tree.into_iter() {
            writeln!(out, "{:indent$}{name} {version}", "", indent = indent, name = pkg.name(), version = pkg.version())?;
            print_package_tree(out, indent + 2, tree)?;
        }
        Ok(())
    }

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

            print_package_tree(&mut outlock, 0, tree)
        })
        .collect::<Result<()>>()
}


