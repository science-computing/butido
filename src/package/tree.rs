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
use std::borrow::Cow;
use std::io::Result as IoResult;

use anyhow::Result;
use anyhow::anyhow;
use indicatif::ProgressBar;
use log::trace;
use resiter::AndThen;
use serde::Deserialize;
use serde::Serialize;
use ptree::TreeItem;
use ptree::Style;

use crate::package::Package;
use crate::repository::Repository;

#[derive(Debug, Serialize, Deserialize)]
pub struct Tree {
    root: Vec<Mapping>,
}

/// Helper type
///
/// This helper type is required so that the serialized JSON is a bit more readable.
#[derive(Debug, Serialize, Deserialize)]
struct Mapping {
    package: Package,
    dependencies: Tree,
}

impl Tree {

    pub fn new() -> Self {
        Tree { root: Vec::new() }
    }

    pub fn add_package(&mut self, p: Package, repo: &Repository, progress: ProgressBar) -> Result<()> {
        macro_rules! mk_add_package_tree {
            ($this:ident, $pack:ident, $repo:ident, $root:ident, $progress:ident) => {{
                let mut subtree = Tree::new();
                ($pack).get_self_packaged_dependencies()
                    .and_then_ok(|(name, constr)| {
                        trace!("Dependency: {:?}", name);
                        let pack = ($repo).find_with_version(&name, &constr);
                        trace!("Found: {:?}", pack);

                        if pack.iter().any(|p| ($root).has_package(p)) {
                            return Err(anyhow!("Duplicate version of some package in {:?} found", pack))
                        }
                        trace!("All dependecies available...");

                        pack.into_iter()
                            .map(|p| {
                                ($progress).tick();
                                trace!("Following dependecy: {:?}", p);
                                add_package_tree(&mut subtree, p.clone(), ($repo), ($root), ($progress).clone())
                            })
                            .collect()
                    })
                    .collect::<Result<Vec<()>>>()?;

                trace!("Inserting subtree: {:?} -> {:?}", ($pack), subtree);
                ($this).root.push(Mapping { package: ($pack), dependencies: subtree });
                Ok(())
            }}
        };

        fn add_package_tree(this: &mut Tree, p: Package, repo: &Repository, root: &mut Tree, progress: ProgressBar) -> Result<()> {
            mk_add_package_tree!(this, p, repo, root, progress)
        }

        trace!("Making package Tree for {:?}", p);
        let r = mk_add_package_tree!(self, p, repo, self, progress);
        trace!("Finished makeing package Tree");
        r
    }

    /// Get packages of the tree
    ///
    /// This does not yield packages which are dependencies of this tree node.
    /// It yields only packages for this particular Tree instance.
    pub fn packages(&self) -> impl Iterator<Item = &Package> {
        self.root.iter().map(|mapping| &mapping.package)
    }

    /// Get all packages in the tree by reference
    ///
    /// # Warning
    ///
    /// The order of the packages is _NOT_ guaranteed by the implementation
    pub fn all_packages(&self) -> Vec<&Package> {
        self.root
            .iter()
            .map(|m| m.dependencies.all_packages())
            .flatten()
            .chain(self.root.iter().map(|m| &m.package))
            .collect()
    }

    /// Get dependencies stored in this tree
    pub fn dependencies(&self) -> impl Iterator<Item = &Tree> {
        self.root.iter().map(|mapping| &mapping.dependencies)
    }

    pub fn into_iter(self) -> impl IntoIterator<Item = (Package, Tree)> {
        self.root.into_iter().map(|m| (m.package, m.dependencies))
    }

    pub fn has_package(&self, p: &Package) -> bool {
        let name_eq = |k: &Package| k.name() == p.name();
        self.packages().any(name_eq) || self.dependencies().any(|t| t.has_package(p))
    }

    pub fn display(&self) -> Vec<DisplayTree> {
        self.root.iter().map(DisplayTree).collect()
    }

}

#[derive(Clone)]
pub struct DisplayTree<'a>(&'a Mapping);

impl<'a> TreeItem for DisplayTree<'a> {
    type Child = Self;

    fn write_self<W: Write>(&self, f: &mut W, _: &Style) -> IoResult<()> {
        write!(f, "{} {}", self.0.package.name(), self.0.package.version())
    }

    fn children(&self) -> Cow<[Self::Child]> {
        Cow::from(self.0.dependencies.root.iter().map(DisplayTree).collect::<Vec<_>>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use crate::package::tests::pname;
    use crate::package::tests::pversion;
    use crate::package::tests::package;
    use crate::package::Dependency;
    use crate::package::Dependencies;

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

        let mut tree = Tree::new();
        let r = tree.add_package(p1, &repo, progress.clone());
        assert!(r.is_ok());
    }

    #[test]
    fn test_add_two_packages() {
        let mut btree = BTreeMap::new();

        let p1 = {
            let name = "a";
            let vers = "1";
            let pack = package(name, vers, "https://rust-lang.org", "123");
            btree.insert((pname(name), pversion(vers)), pack.clone());
            pack
        };
        let p2 = {
            let name = "b";
            let vers = "2";
            let pack = package(name, vers, "https://rust-lang.org", "124");
            btree.insert((pname(name), pversion(vers)), pack.clone());
            pack
        };

        let repo = Repository::from(btree);
        let progress = ProgressBar::hidden();

        let mut tree = Tree::new();
        let r = tree.add_package(p1, &repo, progress.clone());
        assert!(r.is_ok());

        let r = tree.add_package(p2, &repo, progress.clone());
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
            btree.insert((pname(name), pversion(vers)), pack.clone());
        }

        {
            let d = Dependency::from(String::from("b =2"));
            let ds = Dependencies::with_runtime_dependency(d);
            p1.set_dependencies(ds);
        }

        let repo = Repository::from(btree);
        let progress = ProgressBar::hidden();

        let mut tree = Tree::new();
        let r = tree.add_package(p1, &repo, progress.clone());
        assert!(r.is_ok());
        assert!(tree.packages().all(|p| *p.name() == pname("a")));
        assert!(tree.packages().all(|p| *p.version() == pversion("1")));

        let subtree: Vec<&Tree> = tree.dependencies().collect();
        assert_eq!(subtree.len(), 1);
        let subtree = subtree[0];
        assert!(subtree.packages().all(|p| *p.name() == pname("b")));
        assert!(subtree.packages().all(|p| *p.version() == pversion("2")));
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
            btree.insert((pname(name), pversion(vers)), pack.clone());
        }

        {
            let name = "p3";
            let vers = "3";
            let pack = package(name, vers, "https://rust-lang.org", "125");
            btree.insert((pname(name), pversion(vers)), pack.clone());
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
            btree.insert((pname(name), pversion(vers)), pack.clone());
        }

        {
            let name = "p5";
            let vers = "5";
            let pack = package(name, vers, "https://rust-lang.org", "129");
            btree.insert((pname(name), pversion(vers)), pack.clone());
        }

        {
            let name = "p6";
            let vers = "66.6.6";
            let pack = package(name, vers, "https://rust-lang.org", "666");
            btree.insert((pname(name), pversion(vers)), pack.clone());
        }

        let repo = Repository::from(btree);
        let progress = ProgressBar::hidden();

        let mut tree = Tree::new();
        let r = tree.add_package(p1, &repo, progress.clone());
        assert!(r.is_ok());
        assert!(tree.packages().all(|p| *p.name() == pname("p1")));
        assert!(tree.packages().all(|p| *p.version() == pversion("1")));

        let subtrees: Vec<&Tree> = tree.dependencies().collect();
        assert_eq!(subtrees.len(), 1);

        let subtree = subtrees[0];
        assert_eq!(subtree.packages().count(), 2);

        assert!(subtree.packages().all(|p| {
            *p.name() == pname("p2") || *p.name() == pname("p4")
        }));

        let subsubtrees: Vec<&Tree> = subtree.dependencies().collect();
        assert_eq!(subsubtrees.len(), 2);

        assert!(subsubtrees.iter().any(|st| {
            st.packages().count() == 1
        }));

        assert!(subsubtrees.iter().any(|st| {
            st.packages().count() == 2
        }));


        assert!(subsubtrees.iter().any(|st| {
            st.packages().all(|p| *p.name() == pname("p3"))
        }));

        assert!(subsubtrees.iter().any(|st| {
            st.packages().all(|p| {
                *p.name() == pname("p5") ||
                *p.name() == pname("p6")
            })
        }));
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
            btree.insert((pname(name), pversion(vers)), pack.clone());
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
            btree.insert((pname(name), pversion(vers)), pack.clone());
        }

        {
            let name = "p3";
            let vers = "3";
            let pack = package(name, vers, "https://rust-lang.org", "125");
            btree.insert((pname(name), pversion(vers)), pack.clone());
        }

        {
            let name = "p3";
            let vers = "1";
            let pack = package(name, vers, "https://rust-lang.org", "128");
            btree.insert((pname(name), pversion(vers)), pack.clone());
        }

        {
            let name = "p3";
            let vers = "3.1";
            let pack = package(name, vers, "https://rust-lang.org", "118");
            btree.insert((pname(name), pversion(vers)), pack.clone());
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
            btree.insert((pname(name), pversion(vers)), pack.clone());
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
            btree.insert((pname(name), pversion(vers)), pack.clone());
        }

        {
            let name = "p5";
            let vers = "5";
            let pack = package(name, vers, "https://rust-lang.org", "129");
            btree.insert((pname(name), pversion(vers)), pack.clone());
        }

        {
            let name = "p6";
            let vers = "66.6.6";
            let pack = package(name, vers, "https://rust-lang.org", "666");
            btree.insert((pname(name), pversion(vers)), pack.clone());
        }

        {
            let name = "p6";
            let vers = "66.6.8";
            let pack = package(name, vers, "https://rust-lang.org", "666");
            btree.insert((pname(name), pversion(vers)), pack.clone());
        }

        let repo = Repository::from(btree);
        let progress = ProgressBar::hidden();

        let mut tree = Tree::new();
        let r = tree.add_package(p1, &repo, progress.clone());
        assert!(r.is_ok());
        assert!(tree.packages().all(|p| *p.name() == pname("p1")));
        assert!(tree.packages().all(|p| *p.version() == pversion("1")));

        let subtrees: Vec<&Tree> = tree.dependencies().collect();
        assert_eq!(subtrees.len(), 1);

        let subtree = subtrees[0];
        assert_eq!(subtree.packages().count(), 2);

        assert!(subtree.packages().all(|p| {
            *p.name() == pname("p2") || *p.name() == pname("p4")
        }));

        let subsubtrees: Vec<&Tree> = subtree.dependencies().collect();
        assert_eq!(subsubtrees.len(), 2);

        assert!(subsubtrees.iter().any(|st| {
            st.packages().count() == 1
        }));

        assert!(subsubtrees.iter().any(|st| {
            st.packages().count() == 2
        }));


        assert!(subsubtrees.iter().any(|st| {
            st.packages().all(|p| *p.name() == pname("p3"))
        }));

        assert!(subsubtrees.iter().any(|st| {
            st.packages().all(|p| {
                *p.name() == pname("p5") ||
                *p.name() == pname("p6")
            })
        }));
    }

}
