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
        progress: Option<&ProgressBar>,
    ) -> Result<Self> {
        fn add_sub_packages<'a>(
            repo: &'a Repository,
            mappings: &mut HashMap<&'a Package, daggy::NodeIndex>,
            dag: &mut daggy::Dag<&'a Package, i8>,
            p: &'a Package,
            progress: Option<&ProgressBar>
        ) -> Result<()> {
            p.get_self_packaged_dependencies()
                .and_then_ok(|(name, constr)| {
                    trace!("Dependency for {} {} found: {:?}", p.name(), p.version(), name);
                    let packs = repo.find_with_version(&name, &constr);
                    if packs.is_empty() {
                        return Err(anyhow!("Dependency of {} {} not found: {} {}", p.name(), p.version(), name, constr))
                    }
                    trace!("Found in repo: {:?}", packs);

                    // If we didn't check that dependency already
                    if !mappings.keys().any(|p| packs.iter().any(|pk| pk.name() == p.name() && pk.version() == p.version())) {
                        // recurse
                        packs.into_iter()
                            .try_for_each(|p| {
                                progress.as_ref().map(|p| p.tick());

                                let idx = dag.add_node(p);
                                mappings.insert(p, idx);

                                trace!("Recursing for: {:?}", p);
                                add_sub_packages(repo, mappings, dag, p, progress)
                            })
                    } else {
                        Ok(())
                    }
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
        add_sub_packages(repo, &mut mappings, &mut dag, &p, progress)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use crate::package::tests::package;
    use crate::package::tests::pname;
    use crate::package::tests::pversion;
    use crate::package::Dependencies;
    use crate::package::Dependency;

    use indicatif::ProgressBar;

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

        let r = Dag::for_root_package(p1, &repo, &progress);
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

        let dag = Dag::for_root_package(p1, &repo, &progress);
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

        let r = Dag::for_root_package(p1, &repo, &progress);
        assert!(r.is_ok());
        let r = r.unwrap();
        let ps = r.all_packages();
        assert!(ps.iter().any(|p| *p.name() == pname("p1") && *p.version() == pversion("1")));
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

        let r = Dag::for_root_package(p1, &repo, &progress);
        assert!(r.is_ok());
        let r = r.unwrap();
        let ps = r.all_packages();
        assert!(ps.iter().any(|p| *p.name() == pname("p1") && *p.version() == pversion("1")));
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

        let r = Dag::for_root_package(p1, &repo, &progress);
        assert!(r.is_ok());
        let r = r.unwrap();
        let ps = r.all_packages();
        assert!(ps.iter().any(|p| *p.name() == pname("p1") && *p.version() == pversion("1")));
        assert!(ps.iter().any(|p| *p.name() == pname("p2")));
        assert!(ps.iter().any(|p| *p.name() == pname("p3")));
        assert!(ps.iter().any(|p| *p.name() == pname("p4")));
    }
}

