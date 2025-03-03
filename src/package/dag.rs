//
// Copyright (c) 2020-2022 science+computing ag and other contributors
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

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use getset::Getters;
use indicatif::ProgressBar;
use itertools::Itertools;
use petgraph::acyclic::Acyclic;
use petgraph::data::Build;
use petgraph::graph::DiGraph;
use petgraph::graph::EdgeIndex;
use petgraph::graph::NodeIndex;
use ptree::Style;
use ptree::TreeItem;
use resiter::AndThen;
use tracing::trace;

use crate::package::condition::ConditionCheckable;
use crate::package::condition::ConditionData;
use crate::package::dependency::ParseDependency;
use crate::package::Package;
use crate::package::PackageName;
use crate::package::PackageVersion;
use crate::repository::Repository;

#[derive(Debug, Getters)]
pub struct Dag {
    #[getset(get = "pub")]
    dag: Acyclic<DiGraph<Package, DependencyType>>,

    #[getset(get = "pub")]
    root_idx: NodeIndex,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum DependencyType {
    Build,
    Runtime,
}

impl Dag {
    /// Builds the package/dependency DAG for the given package
    pub fn for_root_package(
        p: Package,
        repo: &Repository,
        progress: Option<&ProgressBar>,
        conditional_data: &ConditionData<'_>, // required for selecting packages with conditional dependencies
    ) -> Result<Self> {
        /// Helper fn to check the dependency condition of a dependency and parse the dependency
        /// into a tuple for further processing
        fn process_dependency<D: ConditionCheckable + ParseDependency>(
            dependency: &D,
            dependency_type: DependencyType,
            conditional_data: &ConditionData<'_>,
        ) -> Result<(bool, PackageName, PackageVersion, DependencyType)> {
            // Check whether the condition of the dependency matches our data
            let take = dependency.check_condition(conditional_data)?;
            let (name, version) = dependency.parse_as_name_and_version()?;

            // (dependency check result, name of the dependency, version of the
            // dependency, and type (build/runtime))
            Ok((take, name, version, dependency_type))
        }

        /// Helper fn to get the dependencies of a package
        ///
        /// This function helps getting the dependencies of a package as an iterator over
        /// (Name, Version).
        ///
        /// It also filters out dependencies that do not match the `conditional_data` passed and
        /// makes the dependencies unique over (name, version).
        fn get_package_dependencies<'a>(
            package: &'a Package,
            conditional_data: &'a ConditionData<'_>,
        ) -> impl Iterator<Item = Result<(PackageName, PackageVersion, DependencyType)>> + 'a
        {
            trace!("Collecting the dependencies of {package:?} {conditional_data:?}");
            package
                .dependencies()
                .build()
                .iter()
                .map(move |d| process_dependency(d, DependencyType::Build, conditional_data))
                .chain({
                    package.dependencies().runtime().iter().map(move |d| {
                        process_dependency(d, DependencyType::Runtime, conditional_data)
                    })
                })
                // Now filter out all dependencies where their condition did not match our
                // `conditional_data`.
                .filter(|res| match res {
                    Ok((true, _, _, _)) => true,
                    Ok((false, _, _, _)) => false,
                    Err(_) => true,
                })
                // Map out the boolean from the condition, because we don't need that later on
                .map(|res| res.map(|(_, name, vers, kind)| (name, vers, kind)))
                // Make all dependencies unique, because we don't want to build one dependency
                // multiple times (TODO: there shouldn't be duplicates -> warn/error instead)
                .unique_by(|res| res.as_ref().ok().cloned())
        }

        /// Main helper function to build the DAG. Recursively resolves a package's dependencies
        /// and adds corresponding nodes to the DAG. The edges are added later in `add_edges()`.
        fn add_sub_packages<'a>(
            repo: &'a Repository,
            mappings: &mut HashMap<&'a Package, NodeIndex>,
            dag: &mut Acyclic<DiGraph<&'a Package, DependencyType>>,
            p: &'a Package,
            progress: Option<&ProgressBar>,
            conditional_data: &ConditionData<'_>,
        ) -> Result<()> {
            get_package_dependencies(p, conditional_data)
                .and_then_ok(|(name, version, kind)| {
                    trace!(
                        "Processing the following dependency of {} {}: {} {} {:?}",
                        p.name(),
                        p.version(),
                        name,
                        version,
                        kind
                    );
                    let packs = repo.find_with_version(&name, &version);
                    trace!(
                        "Found the following matching packages in the repo: {:?}",
                        packs
                    );
                    if packs.is_empty() {
                        anyhow::bail!(
                            "Couldn't find the following dependency of {} {} in the repo: {} {}",
                            p.name(),
                            p.version(),
                            name,
                            version
                        );
                    }

                    // Check if we already created a DAG node for any of the matching packages and
                    // only add a new node and recurse if necessary.
                    if !mappings.keys().any(|p| {
                        packs
                            .iter()
                            .any(|pk| pk.name() == p.name() && pk.version() == p.version())
                    }) {
                        // TODO: It should be sufficient to process a single package of `packs`.
                        // The `packs` vector contains a list of all packages in the repo that
                        // match the dependency specification (PackageName and PackageVersion).
                        // TODO: Support PackageVersionConstraint: All packages must have the same
                        // name so only the version can differ -> we could simply pick the package
                        // with the most recent version and optionally omit a warning (or even
                        // abort with an error).
                        packs.into_iter().try_for_each(|p| {
                            let _ = progress.as_ref().map(|p| p.tick());

                            // Add the package to the DAG and recursively proceed with the
                            // subpackages (dependencies).
                            let idx = dag.add_node(p);
                            mappings.insert(p, idx);

                            trace!("Recursing for: {:?}", p);
                            add_sub_packages(repo, mappings, dag, p, progress, conditional_data)
                        })
                    } else {
                        Ok(())
                    }
                })
                .collect::<Result<()>>()
        }

        // Helper fn to add the edges to the DAG with all nodes.
        // TODO: It seems easier and more efficient to do this in `add_sub_packages` as well (it
        // makes that function more complex but doing it separately is weird).
        fn add_edges(
            mappings: &HashMap<&Package, NodeIndex>,
            dag: &mut Acyclic<DiGraph<&Package, DependencyType>>,
            conditional_data: &ConditionData<'_>,
        ) -> Result<()> {
            for (package, idx) in mappings {
                get_package_dependencies(package, conditional_data)
                    .and_then_ok(|(dep_name, dep_version, dep_kind)| {
                        mappings
                            .iter()
                            .filter(|(pkg, _)| {
                                *pkg.name() == dep_name && *pkg.version() == dep_version
                            })
                            .try_for_each(|(dep, dep_idx)| {
                                dag.try_add_edge(*idx, *dep_idx, dep_kind.clone())
                                    .map(|_| ())
                                    // Only debug formatting is available for the error and for
                                    // cycles it is quite useless (the context below is much more
                                    // helpful than, e.g., "Cycle(Cycle(NodeIndex(0)))") but we'll
                                    // include it for completeness and in case of the other errors
                                    // that could theoretically occur:
                                    .map_err(|e| anyhow!(format!("{e:?}")))
                                    .with_context(|| {
                                        anyhow!(
                                            "Failed to add package dependency DAG edge \
                                            from package \"{}\" ({}) to dependency \"{}\" ({})",
                                            package.name(),
                                            package.version(),
                                            dep.name(),
                                            dep.version(),
                                        )
                                    })
                            })
                    })
                    .collect::<Result<()>>()?
            }

            Ok(())
        }

        // Create an empty DAG and use the above helper functions to compute the dependency graph:
        let mut dag = Acyclic::<DiGraph<&Package, DependencyType>>::new();
        let mut mappings = HashMap::new();

        trace!("Building the package dependency DAG for package {:?}", p);
        let root_idx = dag.add_node(&p);
        mappings.insert(&p, root_idx);
        add_sub_packages(
            repo,
            &mut mappings,
            &mut dag,
            &p,
            progress,
            conditional_data,
        )?;
        trace!("Adding the dependency edges to the DAG for package {:?}", p);
        add_edges(&mappings, &mut dag, conditional_data)?;
        trace!("Finished building the package DAG");

        Ok(Dag {
            dag: Acyclic::<_>::try_from_graph(dag.map(
                |_, p: &&Package| -> Package { (*p).clone() },
                |_, e| (*e).clone(),
            ))
            .unwrap(), // The dag is already acyclic so this cannot fail
            root_idx,
        })
    }

    /// Get all packages in the tree by reference
    ///
    /// # Warning
    ///
    /// The order of the packages is _NOT_ guaranteed by the implementation
    pub fn all_packages(&self) -> Vec<&Package> {
        self.dag
            .node_indices()
            .filter_map(|idx| self.dag.node_weight(idx))
            .collect()
    }

    pub fn display(&self) -> DagDisplay {
        DagDisplay(self, self.root_idx, None)
    }
}

#[derive(Clone)]
pub struct DagDisplay<'a>(&'a Dag, NodeIndex, Option<EdgeIndex>);

impl TreeItem for DagDisplay<'_> {
    type Child = Self;

    fn write_self<W: Write>(&self, f: &mut W, _: &Style) -> IoResult<()> {
        let p = self
            .0
            .dag
            .node_weight(self.1)
            .ok_or_else(|| anyhow!("Error finding node: {:?}", self.1))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let dependency_type = match self.2 {
            // Only the root package has no edge and we pretend it's a runtime dependency as we
            // only mark build time dependencies in the output:
            None => &DependencyType::Runtime,
            Some(edge_idx) => self
                .0
                .dag
                .edge_weight(edge_idx)
                .ok_or_else(|| anyhow!("Error finding edge: {:?}", self.2))
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
        };
        let extra_info = match dependency_type {
            // We mark build time dependencies with a star:
            &DependencyType::Build => "*",
            _ => "",
        };
        write!(f, "{}{} {}", extra_info, p.name(), p.version())
    }

    fn children(&self) -> Cow<[Self::Child]> {
        let mut children_walker = self
            .0
            .dag
            .neighbors_directed(self.1, petgraph::Outgoing)
            .detach();
        let mut children = Vec::<Self::Child>::new();
        while let Some((edge_idx, node_idx)) = children_walker.next(&self.0.dag) {
            children.push(DagDisplay(self.0, node_idx, Some(edge_idx)));
        }
        Cow::from(children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use crate::package::condition::Condition;
    use crate::package::condition::OneOrMore;
    use crate::package::tests::package;
    use crate::package::tests::pname;
    use crate::package::tests::pversion;
    use crate::package::Dependencies;
    use crate::package::Dependency;
    use crate::util::docker::ImageName;

    #[test]
    fn test_add_package() {
        let mut btree = BTreeMap::new();

        let p1 = {
            let name = "a";
            let vers = "1";
            let pack = package(name, vers, "https://rust-lang.org", "123");
            btree.insert((pname(name), pversion(vers)), pack.clone());
            pack
        };

        let repo = Repository::from(btree);
        let progress = ProgressBar::hidden();

        let condition_data = ConditionData {
            image_name: None,
            env: &[],
        };

        let r = Dag::for_root_package(p1, &repo, Some(&progress), &condition_data);

        assert!(r.is_ok());
    }

    #[test]
    fn test_add_two_dependent_packages() {
        let mut btree = BTreeMap::new();

        let mut p1 = {
            let name = "a";
            let vers = "1";
            let pack = package(name, vers, "https://rust-lang.org", "123");
            btree.insert((pname(name), pversion(vers)), pack.clone());
            pack
        };

        {
            let name = "b";
            let vers = "2";
            let pack = package(name, vers, "https://rust-lang.org", "124");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let d = Dependency::from(String::from("b =2"));
            let ds = Dependencies::with_runtime_dependency(d);
            p1.set_dependencies(ds);
        }

        let repo = Repository::from(btree);
        let progress = ProgressBar::hidden();

        let condition_data = ConditionData {
            image_name: None,
            env: &[],
        };

        let dag = Dag::for_root_package(p1, &repo, Some(&progress), &condition_data);
        assert!(dag.is_ok());
        let dag = dag.unwrap();
        let ps = dag.all_packages();

        assert!(ps.iter().any(|p| *p.name() == pname("a")));
        assert!(ps.iter().any(|p| *p.version() == pversion("1")));
        assert!(ps.iter().any(|p| *p.name() == pname("b")));
        assert!(ps.iter().any(|p| *p.version() == pversion("2")));
    }

    #[test]
    fn test_add_deep_package_tree() {
        let mut btree = BTreeMap::new();

        //
        // Test the following (made up) tree:
        //
        //  p1
        //   - p2
        //     - p3
        //   - p4
        //     - p5
        //     - p6
        //

        let p1 = {
            let name = "p1";
            let vers = "1";
            let mut pack = package(name, vers, "https://rust-lang.org", "123");
            {
                let d1 = Dependency::from(String::from("p2 =2"));
                let d2 = Dependency::from(String::from("p4 =4"));
                let ds = Dependencies::with_runtime_dependencies(vec![d1, d2]);
                pack.set_dependencies(ds);
            }
            btree.insert((pname(name), pversion(vers)), pack.clone());
            pack
        };

        {
            let name = "p2";
            let vers = "2";
            let mut pack = package(name, vers, "https://rust-lang.org", "124");
            {
                let d1 = Dependency::from(String::from("p3 =3"));
                let ds = Dependencies::with_runtime_dependencies(vec![d1]);
                pack.set_dependencies(ds);
            }
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p3";
            let vers = "3";
            let pack = package(name, vers, "https://rust-lang.org", "125");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p4";
            let vers = "4";
            let mut pack = package(name, vers, "https://rust-lang.org", "125");
            {
                let d1 = Dependency::from(String::from("p5 =5"));
                let d2 = Dependency::from(String::from("p6 =66.6.6"));
                let ds = Dependencies::with_runtime_dependencies(vec![d1, d2]);
                pack.set_dependencies(ds);
            }
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p5";
            let vers = "5";
            let pack = package(name, vers, "https://rust-lang.org", "129");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p6";
            let vers = "66.6.6";
            let pack = package(name, vers, "https://rust-lang.org", "666");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);
        let progress = ProgressBar::hidden();

        let condition_data = ConditionData {
            image_name: None,
            env: &[],
        };

        let r = Dag::for_root_package(p1, &repo, Some(&progress), &condition_data);
        assert!(r.is_ok());
        let r = r.unwrap();
        let ps = r.all_packages();
        assert!(ps
            .iter()
            .any(|p| *p.name() == pname("p1") && *p.version() == pversion("1")));
        assert!(ps.iter().any(|p| *p.name() == pname("p2")));
        assert!(ps.iter().any(|p| *p.name() == pname("p4")));
        assert!(ps.iter().any(|p| *p.name() == pname("p3")));
        assert!(ps.iter().any(|p| *p.name() == pname("p5")));
        assert!(ps.iter().any(|p| *p.name() == pname("p6")));
    }

    #[test]
    fn test_add_deep_package_tree_with_irrelevant_packages() {
        // this is the same test as test_add_deep_package_tree(), but with a bunch of irrelevant
        // packages added to the repository, so that we can be sure that the algorithm finds the
        // actually required packages
        //
        // The irrelevant packages are all packages that already exist, but with different versions

        let mut btree = BTreeMap::new();

        //
        // Test the following (made up) tree:
        //
        //  p1
        //   - p2
        //     - p3
        //   - p4
        //     - p5
        //     - p6
        //

        let p1 = {
            let name = "p1";
            let vers = "1";
            let mut pack = package(name, vers, "https://rust-lang.org", "123");
            {
                let d1 = Dependency::from(String::from("p2 =2"));
                let d2 = Dependency::from(String::from("p4 =4"));
                let ds = Dependencies::with_runtime_dependencies(vec![d1, d2]);
                pack.set_dependencies(ds);
            }
            btree.insert((pname(name), pversion(vers)), pack.clone());
            pack
        };

        {
            let name = "p1";
            let vers = "2";
            let mut pack = package(name, vers, "https://rust-lang.org", "123");
            {
                let d1 = Dependency::from(String::from("p2 =2"));
                let d2 = Dependency::from(String::from("p4 =5"));
                let ds = Dependencies::with_runtime_dependencies(vec![d1, d2]);
                pack.set_dependencies(ds);
            }
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p2";
            let vers = "2";
            let mut pack = package(name, vers, "https://rust-lang.org", "124");
            {
                let d1 = Dependency::from(String::from("p3 =3"));
                let ds = Dependencies::with_runtime_dependencies(vec![d1]);
                pack.set_dependencies(ds);
            }
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p3";
            let vers = "3";
            let pack = package(name, vers, "https://rust-lang.org", "125");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p3";
            let vers = "1";
            let pack = package(name, vers, "https://rust-lang.org", "128");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p3";
            let vers = "3.1";
            let pack = package(name, vers, "https://rust-lang.org", "118");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p4";
            let vers = "4";
            let mut pack = package(name, vers, "https://rust-lang.org", "125");
            {
                let d1 = Dependency::from(String::from("p5 =5"));
                let d2 = Dependency::from(String::from("p6 =66.6.6"));
                let ds = Dependencies::with_runtime_dependencies(vec![d1, d2]);
                pack.set_dependencies(ds);
            }
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p4";
            let vers = "5";
            let mut pack = package(name, vers, "https://rust-lang.org", "125");
            {
                let d1 = Dependency::from(String::from("p5 =5"));
                let d2 = Dependency::from(String::from("p6 =66.6.8"));
                let ds = Dependencies::with_runtime_dependencies(vec![d1, d2]);
                pack.set_dependencies(ds);
            }
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p5";
            let vers = "5";
            let pack = package(name, vers, "https://rust-lang.org", "129");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p6";
            let vers = "66.6.6";
            let pack = package(name, vers, "https://rust-lang.org", "666");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p6";
            let vers = "66.6.8";
            let pack = package(name, vers, "https://rust-lang.org", "666");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);
        let progress = ProgressBar::hidden();

        let condition_data = ConditionData {
            image_name: None,
            env: &[],
        };

        let r = Dag::for_root_package(p1, &repo, Some(&progress), &condition_data);
        assert!(r.is_ok());
        let r = r.unwrap();
        let ps = r.all_packages();
        assert!(ps
            .iter()
            .any(|p| *p.name() == pname("p1") && *p.version() == pversion("1")));
        assert!(ps.iter().any(|p| *p.name() == pname("p2")));
        assert!(ps.iter().any(|p| *p.name() == pname("p3")));
        assert!(ps.iter().any(|p| *p.name() == pname("p4")));
        assert!(ps.iter().any(|p| *p.name() == pname("p5")));
        assert!(ps.iter().any(|p| *p.name() == pname("p6")));
    }

    #[test]
    fn test_add_dag() {
        let mut btree = BTreeMap::new();

        //
        // Test the following (made up) tree:
        //
        //  p1
        //   - p2
        //     - p3
        //   - p4
        //     - p3
        //
        // where "p3" is referenced from "p2" and "p4"
        //
        // The tree also contains a few irrelevant packages.
        //

        let p1 = {
            let name = "p1";
            let vers = "1";
            let mut pack = package(name, vers, "https://rust-lang.org", "123");
            {
                let d1 = Dependency::from(String::from("p2 =2"));
                let d2 = Dependency::from(String::from("p4 =4"));
                let ds = Dependencies::with_runtime_dependencies(vec![d1, d2]);
                pack.set_dependencies(ds);
            }
            btree.insert((pname(name), pversion(vers)), pack.clone());
            pack
        };

        {
            let name = "p1";
            let vers = "2";
            let mut pack = package(name, vers, "https://rust-lang.org", "123");
            {
                let d1 = Dependency::from(String::from("p2 =2"));
                let d2 = Dependency::from(String::from("p4 =5"));
                let ds = Dependencies::with_runtime_dependencies(vec![d1, d2]);
                pack.set_dependencies(ds);
            }
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p2";
            let vers = "2";
            let mut pack = package(name, vers, "https://rust-lang.org", "124");
            {
                let d1 = Dependency::from(String::from("p3 =3"));
                let ds = Dependencies::with_runtime_dependencies(vec![d1]);
                pack.set_dependencies(ds);
            }
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p3";
            let vers = "3";
            let pack = package(name, vers, "https://rust-lang.org", "125");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p3";
            let vers = "1";
            let pack = package(name, vers, "https://rust-lang.org", "128");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p3";
            let vers = "3.1";
            let pack = package(name, vers, "https://rust-lang.org", "118");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "p4";
            let vers = "4";
            let mut pack = package(name, vers, "https://rust-lang.org", "125");
            {
                let d1 = Dependency::from(String::from("p3 =3"));
                let ds = Dependencies::with_runtime_dependencies(vec![d1]);
                pack.set_dependencies(ds);
            }
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);
        let progress = ProgressBar::hidden();

        let condition_data = ConditionData {
            image_name: None,
            env: &[],
        };

        let r = Dag::for_root_package(p1, &repo, Some(&progress), &condition_data);
        assert!(r.is_ok());
        let r = r.unwrap();
        let ps = r.all_packages();
        assert!(ps
            .iter()
            .any(|p| *p.name() == pname("p1") && *p.version() == pversion("1")));
        assert!(ps.iter().any(|p| *p.name() == pname("p2")));
        assert!(ps.iter().any(|p| *p.name() == pname("p3")));
        assert!(ps.iter().any(|p| *p.name() == pname("p4")));
    }

    /// Build a repository with two packages and a condition for their dependency
    fn repo_with_ab_packages_with_condition(cond: Condition) -> (Package, Repository) {
        let mut btree = BTreeMap::new();

        let mut p1 = {
            let name = "a";
            let vers = "1";
            let pack = package(name, vers, "https://rust-lang.org", "123");
            btree.insert((pname(name), pversion(vers)), pack.clone());
            pack
        };

        {
            let name = "b";
            let vers = "2";
            let pack = package(name, vers, "https://rust-lang.org", "124");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let d = Dependency::new_conditional(String::from("b =2"), cond);
            let ds = Dependencies::with_runtime_dependency(d);
            p1.set_dependencies(ds);
        }

        (p1, Repository::from(btree))
    }

    // Test whether the dependency DAG is correctly build if there is NO conditional data passed
    //
    // Because the dependency is conditional with "fooimage" required as build-image, the
    // dependency DAG should NOT contain package "b"
    #[test]
    fn test_add_two_dependent_packages_with_image_conditional() {
        let condition = {
            let in_image = Some(OneOrMore::<String>::One(String::from("fooimage")));
            Condition::new(None, None, in_image)
        };
        let (p1, repo) = repo_with_ab_packages_with_condition(condition);

        let condition_data = ConditionData {
            image_name: None,
            env: &[],
        };

        let progress = ProgressBar::hidden();

        let dag = Dag::for_root_package(p1, &repo, Some(&progress), &condition_data);
        assert!(dag.is_ok());
        let dag = dag.unwrap();
        let ps = dag.all_packages();

        assert!(ps.iter().any(|p| *p.name() == pname("a")));
        assert!(ps.iter().any(|p| *p.version() == pversion("1")));

        // Not in the tree:
        assert!(
            !ps.iter().any(|p| *p.name() == pname("b")),
            "'b' should not be in tree, but is: {ps:?}"
        );
        assert!(
            !ps.iter().any(|p| *p.version() == pversion("2")),
            "'2' should not be in tree, but is: {ps:?}"
        );
    }

    // Test whether the dependency DAG is correctly build if a image is used, but not the one
    // required
    //
    // Because the dependency is conditional with "fooimage" required as build-image, but
    // "barimage" is used, the dependency DAG should NOT contain package "b"
    #[test]
    fn test_add_two_dependent_packages_with_image_conditional_but_other_image_provided() {
        let condition = {
            let in_image = Some(OneOrMore::<String>::One(String::from("fooimage")));
            Condition::new(None, None, in_image)
        };
        let (p1, repo) = repo_with_ab_packages_with_condition(condition);

        let img_name = ImageName::from("barimage");
        let condition_data = ConditionData {
            image_name: Some(&img_name),
            env: &[],
        };

        let progress = ProgressBar::hidden();

        let dag = Dag::for_root_package(p1, &repo, Some(&progress), &condition_data);
        assert!(dag.is_ok());
        let dag = dag.unwrap();
        let ps = dag.all_packages();

        assert!(ps.iter().any(|p| *p.name() == pname("a")));
        assert!(ps.iter().any(|p| *p.version() == pversion("1")));

        // Not in the tree:
        assert!(!ps.iter().any(|p| *p.name() == pname("b")));
        assert!(!ps.iter().any(|p| *p.version() == pversion("2")));
    }

    // Test whether the dependency DAG is correctly build if the right image name is passed
    #[test]
    fn test_add_two_dependent_packages_with_image_conditional_and_image_provided() {
        let condition = {
            let in_image = Some(OneOrMore::<String>::One(String::from("fooimage")));
            Condition::new(None, None, in_image)
        };
        let (p1, repo) = repo_with_ab_packages_with_condition(condition);

        let img_name = ImageName::from("fooimage");
        let condition_data = ConditionData {
            image_name: Some(&img_name),
            env: &[],
        };

        let progress = ProgressBar::hidden();

        let dag = Dag::for_root_package(p1, &repo, Some(&progress), &condition_data);
        assert!(dag.is_ok());
        let dag = dag.unwrap();
        let ps = dag.all_packages();

        assert!(ps.iter().any(|p| *p.name() == pname("a")));
        assert!(ps.iter().any(|p| *p.version() == pversion("1")));

        // IN the tree:
        assert!(ps.iter().any(|p| *p.name() == pname("b")));
        assert!(ps.iter().any(|p| *p.version() == pversion("2")));
    }
}
