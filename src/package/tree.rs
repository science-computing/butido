use std::collections::BTreeMap;

use anyhow::Result;
use anyhow::anyhow;
use indicatif::ProgressBar;

use crate::repository::Repository;
use crate::package::Package;
use crate::util::executor::Executor;

#[derive(Debug)]
pub struct Tree {
    root: BTreeMap<Package, Tree>,
}

impl Tree {

    pub fn new() -> Self {
        Tree { root: BTreeMap::new() }
    }

    pub fn add_package(&mut self, p: Package, repo: &Repository, executor: &dyn Executor, progress: &ProgressBar) -> Result<()> {
        macro_rules! mk_add_package_tree {
            ($this:ident, $pack:ident, $repo:ident, $root:ident, $executor:ident, $progress:ident) => {{
                let mut subtree = Tree::new();
                ($pack).get_all_dependencies($executor)?
                    .into_iter()
                    .map(|(name, constr)| {
                        trace!("Dependency: {:?}", name);
                        let pack = ($repo).find_with_version_constraint(&name, &constr);
                        trace!("Found: {:?}", pack);

                        if pack.iter().any(|p| ($root).has_package(p)) {
                            // package already exists in tree, which is unfortunate
                            // TODO: Handle gracefully
                            //
                            return Err(anyhow!("Duplicate version of some package in {:?} found", pack))
                        }
                        trace!("All dependecies available...");

                        pack.into_iter()
                            .map(|p| {
                                ($progress).tick();
                                trace!("Following dependecy: {:?}", p);
                                add_package_tree(&mut subtree, p.clone(), ($repo), ($root), ($executor), ($progress))
                            })
                            .collect()
                    })
                    .collect::<Result<Vec<()>>>()?;

                trace!("Inserting subtree: {:?}", subtree);
                ($this).root.insert(($pack), subtree);
                Ok(())
            }}
        };

        fn add_package_tree(this: &mut Tree, p: Package, repo: &Repository, root: &mut Tree, executor: &dyn Executor, progress: &ProgressBar) -> Result<()> {
            mk_add_package_tree!(this, p, repo, root, executor, progress)
        }

        trace!("Making package Tree for {:?}", p);
        mk_add_package_tree!(self, p, repo, self, executor, progress)
    }

    /// Get packages of the tree
    ///
    /// This does not yield packages which are dependencies of this tree node.
    /// It yields only packages for this particular Tree instance.
    pub fn packages(&self) -> impl Iterator<Item = &Package> {
        self.root.keys()
    }

    /// Get dependencies stored in this tree
    pub fn dependencies(&self) -> impl Iterator<Item = &Tree> {
        self.root.values()
    }

    pub fn has_package(&self, p: &Package) -> bool {
        let name_eq = |k: &Package| k.name() == p.name();
        self.root.keys().any(name_eq) || self.root.values().any(|t| t.has_package(p))
    }

    /// Find how deep the package is in the tree
    ///
    /// # Returns
    ///
    /// * None if the package is not in the tree
    /// * Some(usize) with the depth of the package in the tree, where the package at the root of
    /// the tree is treated as 0 (zero)
    ///
    /// # Note
    ///
    /// If the package is multiple times in the tree, only the first one will be found
    // TODO: Remove allow(unused)
    #[allow(unused)]
    pub fn package_depth(&self, p: &Package) -> Option<usize> {
        self.package_depth_where(|k| k == p)
    }

    /// Same as `package_depth()` but with custom compare functionfunction
    // TODO: Remove allow(unused)
    #[allow(unused)]
    pub fn package_depth_where<F>(&self, cmp: F) -> Option<usize>
        where F: Fn(&Package) -> bool
    {
        fn find_package_depth<F>(tree: &Tree, current: usize, cmp: &F) -> Option<usize>
            where F: Fn(&Package) -> bool
        {
            if tree.root.keys().any(|k| cmp(k)) {
                return Some(current)
            } else {
                tree.root
                    .values()
                    .filter_map(|subtree| find_package_depth(subtree, current + 1, cmp))
                    .next()
            }
        }

        find_package_depth(self, 0, &cmp)
    }

    pub fn debug_print(&self, sink: &mut dyn std::io::Write) -> std::result::Result<(), std::io::Error> {
        fn print_recursive(tree: &Tree, sink: &mut dyn std::io::Write, indent: usize) -> std::result::Result<(), std::io::Error> {
            for (k, v) in tree.root.iter() {
                writeln!(sink, "{:indent$}- {:?}", " ", k, indent = indent*2)?;
                print_recursive(v, sink, indent + 1)?;
            }

            Ok(())
        }

        print_recursive(self, sink, 0)
    }

}

