use std::collections::BTreeMap;

use anyhow::Result;
use anyhow::anyhow;

use crate::package::Package;
use crate::package::Loader;

pub struct Tree {
    root: BTreeMap<Package, Tree>,
}

impl Tree {

    pub fn new() -> Self {
        Tree { root: BTreeMap::new() }
    }

    pub fn add_package(&mut self, p: Package, loader: &Loader) -> Result<()> {
        macro_rules! mk_add_package_tree {
            ($this:ident, $pack:ident, $loader:ident, $root:ident) => {{
                let mut subtree = Tree::new();
                ($pack).get_all_dependencies()?
                    .into_iter()
                    .map(|(name, constr)| {
                        let pack = ($loader)
                            .load(&name, &constr)?
                            .ok_or_else(|| anyhow!("Package not found: {}", name))?;

                        if ($root).has_package(&($pack)) {
                            // package already exists in tree, which is unfortunate
                            // TODO: Handle gracefully
                            //
                            return Err(anyhow!("Duplicate version of package {:?} found", ($pack)))
                        }

                        add_package_tree(&mut subtree, pack, ($loader), ($root))
                    })
                    .collect::<Result<Vec<()>>>()?;

                ($this).root.insert(($pack), subtree);
                Ok(())
            }}
        };

        fn add_package_tree(this: &mut Tree, p: Package, loader: &Loader, root: &mut Tree) -> Result<()> {
            mk_add_package_tree!(this, p, loader, root)
        }

        mk_add_package_tree!(self, p, loader, self)
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
    ///
    pub fn package_depth(&self, p: &Package) -> Option<usize> {
        self.package_depth_of(p, |k| k == p)
    }

    /// Same as `package_depth()` but with custom compare functionfunction
    pub fn package_depth_of<F>(&self, p: &Package, cmp: F) -> Option<usize>
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

}
