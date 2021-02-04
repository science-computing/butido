//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Result as IoResult;
use std::io::Write;

use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use daggy::Walker;
use indicatif::ProgressBar;
use log::trace;
use ptree::Style;
use ptree::TreeItem;
use resiter::AndThen;
use getset::Getters;

use crate::package::Package;
use crate::repository::Repository;

#[derive(Debug, Getters)]
pub struct Dag {
    #[getset(get = "pub")]
    dag: daggy::Dag<Package, i8>,

    #[getset(get = "pub")]
    root_idx: daggy::NodeIndex,
}

impl Dag {
    pub fn for_root_package(
        p: Package,
        repo: &Repository,
        progress: ProgressBar,
    ) -> Result<Self> {
        fn add_sub_packages<'a>(
            repo: &'a Repository,
            mappings: &mut HashMap<&'a Package, daggy::NodeIndex>,
            dag: &mut daggy::Dag<&'a Package, i8>,
            p: &'a Package,
            progress: &ProgressBar
        ) -> Result<()> {
            p.get_self_packaged_dependencies()
                .and_then_ok(|(name, constr)| {
                    trace!("Dependency: {:?}", name);
                    let packs = repo.find_with_version(&name, &constr);
                    trace!("Found: {:?}", packs);

                    if mappings.keys().any(|p| packs.iter().any(|pk| pk.name() == p.name() && pk.version() == p.version())) {
                        return Err(anyhow!(
                            "Duplicate version of some package in {:?} found",
                            packs
                        ));
                    }
                    trace!("All dependecies available...");

                    packs.into_iter()
                        .map(|p| {
                            progress.tick();
                            trace!("Following dependecy: {:?}", p);

                            let idx = dag.add_node(p);
                            mappings.insert(p, idx);
                            add_sub_packages(repo, mappings, dag, p, progress)
                        })
                        .collect()
                })
                .collect::<Result<()>>()
        }

        fn add_edges(mappings: &HashMap<&Package, daggy::NodeIndex>, dag: &mut daggy::Dag<&Package, i8>) -> Result<()> {
            for (package, idx) in mappings {
                package.get_self_packaged_dependencies()
                    .and_then_ok(|(name, constr)| {
                        mappings
                            .iter()
                            .filter(|(package, _)| *package.name() == name && constr.matches(package.version()))
                            .try_for_each(|(_, dep_idx)| {
                                dag.add_edge(*idx, *dep_idx, 0)
                                    .map(|_| ())
                                    .map_err(Error::from)
                            })
                    })
                    .collect::<Result<()>>()?
            }

            Ok(())
        }

        let mut dag: daggy::Dag<&Package, i8> = daggy::Dag::new();
        let mut mappings = HashMap::new();

        trace!("Making package Tree for {:?}", p);
        let root_idx = dag.add_node(&p);
        mappings.insert(&p, root_idx);
        add_sub_packages(repo, &mut mappings, &mut dag, &p, &progress)?;
        add_edges(&mappings, &mut dag)?;
        trace!("Finished makeing package Tree");

        Ok(Dag {
            dag: dag.map(|_, p: &&Package| -> Package { (*p).clone() }, |_, e| *e),
            root_idx
        })
    }

    /// Get all packages in the tree by reference
    ///
    /// # Warning
    ///
    /// The order of the packages is _NOT_ guaranteed by the implementation
    pub fn all_packages(&self) -> Vec<&Package> {
        self.dag
            .graph()
            .node_indices()
            .filter_map(|idx| self.dag.graph().node_weight(idx))
            .collect()
    }

    pub fn display(&self) -> DagDisplay {
        DagDisplay(self, self.root_idx)
    }
}

#[derive(Clone)]
pub struct DagDisplay<'a>(&'a Dag, daggy::NodeIndex);

impl<'a> TreeItem for DagDisplay<'a> {
    type Child = Self;

    fn write_self<W: Write>(&self, f: &mut W, _: &Style) -> IoResult<()> {
        let p = self.0.dag.graph().node_weight(self.1)
            .ok_or_else(|| anyhow!("Error finding node: {:?}", self.1))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        write!(f, "{} {}", p.name(), p.version())
    }

    fn children(&self) -> Cow<[Self::Child]> {
        let c = self.0.dag.children(self.1);
        Cow::from(c.iter(&self.0.dag)
            .map(|(_, idx)| DagDisplay(self.0, idx))
            .collect::<Vec<_>>()
        )
    }
}

