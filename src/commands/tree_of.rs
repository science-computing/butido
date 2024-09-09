//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'tree-of' subcommand

use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use petgraph::dot::Dot;
use petgraph::graph::DiGraph;
use resiter::AndThen;

use crate::config::Configuration;
use crate::package::condition::ConditionData;
use crate::package::Dag;
use crate::package::DependencyType;
use crate::package::Package;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;
use crate::util::docker::ImageNameLookup;
use crate::util::EnvironmentVariableName;

/// Implementation of the "tree_of" subcommand
pub async fn tree_of(matches: &ArgMatches, repo: Repository, config: &Configuration) -> Result<()> {
    let pname = matches
        .get_one::<String>("package_name")
        .map(|s| s.to_owned())
        .map(PackageName::from);
    let pvers = matches
        .get_one::<String>("package_version")
        .map(|s| s.to_owned())
        .map(PackageVersionConstraint::try_from)
        .transpose()?;

    let image_name_lookup = ImageNameLookup::create(config.docker().images())?;
    let image_name = matches
        .get_one::<String>("image")
        .map(|s| image_name_lookup.expand(s))
        .transpose()?;

    let additional_env = matches
        .get_many::<String>("env")
        .unwrap_or_default()
        .map(AsRef::as_ref)
        .map(crate::util::env::parse_to_env)
        .collect::<Result<Vec<(EnvironmentVariableName, String)>>>()?;

    let condition_data = ConditionData {
        image_name: image_name.as_ref(),
        env: &additional_env,
    };

    let dot = matches.get_flag("dot");

    let serial_buildorder = matches.get_flag("serial-buildorder");

    repo.packages()
        .filter(|p| pname.as_ref().map(|n| p.name() == n).unwrap_or(true))
        .filter(|p| {
            pvers
                .as_ref()
                .map(|v| v.matches(p.version()))
                .unwrap_or(true)
        })
        .map(|package| Dag::for_root_package(package.clone(), &repo, None, &condition_data))
        .and_then_ok(|dag| {
            if dot {
                let petgraph: DiGraph<Package, DependencyType> = (*dag.dag()).clone().into();

                let dot = Dot::with_attr_getters(
                    &petgraph,
                    &[
                        petgraph::dot::Config::EdgeNoLabel,
                        petgraph::dot::Config::NodeNoLabel,
                    ],
                    &|_, er| {
                        format!(
                            "{} ",
                            match er.weight() {
                                DependencyType::Build => "style = \"dotted\"",
                                DependencyType::Runtime => "",
                            }
                        )
                    },
                    &|_, node| format!("label = \"{}\" ", node.1.display_name_version()),
                );

                println!("{:?}", dot);
                Ok(())
            } else if serial_buildorder {
                let petgraph: DiGraph<Package, DependencyType> = (*dag.dag()).clone().into();

                let topo_sorted = petgraph::algo::toposort(&petgraph, None)
                    .map_err(|_| Error::msg("Cyclic dependency found!"))?;

                for node in topo_sorted.iter().rev() {
                    let package = petgraph.node_weight(*node).unwrap();
                    println!("{}", package.clone().display_name_version());
                }
                println!();

                Ok(())
            } else {
                let stdout = std::io::stdout();
                let mut outlock = stdout.lock();

                ptree::write_tree(&dag.display(), &mut outlock).map_err(Error::from)
            }
        })
        .collect::<Result<()>>()
}
