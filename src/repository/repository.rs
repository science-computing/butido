//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use log::trace;
use resiter::FilterMap;
use resiter::Map;

use crate::package::Package;
use crate::package::PackageName;
use crate::package::PackageVersion;
use crate::package::PackageVersionConstraint;

/// A repository represents a collection of packages
pub struct Repository {
    inner: BTreeMap<(PackageName, PackageVersion), Package>,
}

#[cfg(test)]
impl From<BTreeMap<(PackageName, PackageVersion), Package>> for Repository {
    fn from(inner: BTreeMap<(PackageName, PackageVersion), Package>) -> Self {
        Repository { inner }
    }
}

impl Repository {
    fn new(inner: BTreeMap<(PackageName, PackageVersion), Package>) -> Self {
        Repository { inner }
    }

    pub fn load(path: &Path, progress: &indicatif::ProgressBar) -> Result<Self> {
        use crate::repository::fs::FileSystemRepresentation;
        use config::Config;
        use rayon::iter::IntoParallelRefIterator;
        use rayon::iter::ParallelIterator;

        trace!("Loading files from filesystem");
        let fsr = FileSystemRepresentation::load(path.to_path_buf())?;

        fn get_patches(config: &Config) -> Result<Vec<PathBuf>> {
            match config.get_array("patches") {
                Ok(v)  => v.into_iter()
                    .map(config::Value::into_str)
                    .map_err(Error::from)
                    .map_err(|e| e.context("patches must be strings"))
                    .map_err(Error::from)
                    .map_ok(PathBuf::from)
                    .collect(),
                Err(config::ConfigError::NotFound(_)) => Ok(Vec::with_capacity(0)),
                Err(e) => Err(e).map_err(Error::from),
            }
        }

        fsr.files()
            .par_iter()
            .inspect(|path| trace!("Checking for leaf file: {}", path.display()))
            .filter_map(|path| {
                match fsr.is_leaf_file(path) {
                    Ok(true) => Some(Ok(path)),
                    Ok(false) => None,
                    Err(e) => Some(Err(e)),
                }
            })
            .inspect(|r| trace!("Loading files for {:?}", r))
            .map(|path| {
                progress.tick();
                let path = path?;
                fsr.get_files_for(path)?
                    .iter()
                    .inspect(|(path, _)| trace!("Loading layer at {}", path.display()))
                    .fold(Ok(Config::default()) as Result<_>, |config, (path, ref content)| {
                        let mut config = config?;
                        let patches_before_merge = get_patches(&config)?;

                        config.merge(config::File::from_str(&content, config::FileFormat::Toml))
                            .with_context(|| anyhow!("Loading contents of {}", path.display()))?;

                        // get the patches that are in the `config` object after the merge
                        let patches = get_patches(&config)?
                            .into_iter()
                            .map(|p| {
                                if let Some(current_dir) = path.parent() {
                                    fsr.root().join(current_dir).join(p)
                                } else {
                                    unimplemented!()
                                }
                            })
                            .inspect(|patch| trace!("Patch: {:?}", patch))

                            // if the patch file exists, use it (as config::Value).
                            //
                            // Otherwise we have an error here, because we're refering to a non-existing file.
                            .map(|patch| if patch.exists() {
                                trace!("Path to patch exists: {}", patch.display());
                                Ok(Some(patch))
                            } else if patches_before_merge.iter().any(|pb| pb.file_name() == patch.file_name()) {
                                // We have a patch already in the array that is named equal to the patch
                                // we have in the fold iteration.
                                // It seems like this patch was already in the list and we re-found it
                                // because we loaded a "deeper" pkg.toml file.
                                Ok(None)
                            } else {
                                trace!("Path to patch does not exist: {}", patch.display());
                                Err(anyhow!("Patch does not exist: {}", patch.display()))
                            })
                            .filter_map_ok(|o| o)
                            .collect::<Result<Vec<_>>>()?;

                        // If we found any patches, use them. Otherwise use the array from before the merge
                        // (which already has the correct pathes from the previous recursion).
                        let patches = if !patches.is_empty() && patches.iter().all(|p| p.exists()) {
                            patches
                        } else {
                            patches_before_merge
                        };

                        trace!("Patches after postprocessing merge: {:?}", patches);
                        let patches = patches
                            .into_iter()
                            .map(|p| p.display().to_string())
                            .map(config::Value::from)
                            .collect::<Vec<_>>();
                        config.set_once("patches", config::Value::from(patches))?;
                        Ok(config)
                    })
                    .and_then(|c| c.try_into::<Package>().map_err(Error::from))
                    .map(|pkg| ((pkg.name().clone(), pkg.version().clone()), pkg))
            })
            .collect::<Result<BTreeMap<_, _>>>()
            .map(Repository::new)
    }

    pub fn find_by_name<'a>(&'a self, name: &PackageName) -> Vec<&'a Package> {
        trace!("Searching for '{}' in repository", name);
        self.inner
            .iter()
            .filter(|((n, _), _)| {
                trace!("{} == {} -> {}", name, n, name == n);
                name == n
            })
            .map(|(_, pack)| pack)
            .collect()
    }

    pub fn find<'a>(&'a self, name: &PackageName, version: &PackageVersion) -> Vec<&'a Package> {
        self.inner
            .iter()
            .filter(|((n, v), _)| n == name && v == version)
            .map(|(_, p)| p)
            .collect()
    }

    pub fn find_with_version<'a>(
        &'a self,
        name: &PackageName,
        vc: &PackageVersionConstraint,
    ) -> Vec<&'a Package> {
        self.inner
            .iter()
            .filter(|((n, v), _)| n == name && vc.matches(v))
            .map(|(_, p)| p)
            .collect()
    }

    pub fn packages(&self) -> impl Iterator<Item = &Package> {
        self.inner.values()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::package::tests::package;
    use crate::package::tests::pname;
    use crate::package::tests::pversion;

    #[test]
    fn test_finding_by_name() {
        let mut btree = BTreeMap::new();

        {
            let name = "a";
            let vers = "1";
            let pack = package(name, vers, "https://rust-lang.org", "123");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);

        let ps = repo.find_by_name(&pname("a"));
        assert_eq!(ps.len(), 1);

        let p = ps.get(0).unwrap();
        assert_eq!(*p.name(), pname("a"));
        assert_eq!(*p.version(), pversion("1"));
        assert!(!p.version_is_semver());
    }

    #[test]
    fn test_find() {
        let mut btree = BTreeMap::new();

        {
            let name = "a";
            let vers = "1";
            let pack = package(name, vers, "https://rust-lang.org", "123");
            btree.insert((pname(name), pversion(vers)), pack);
        }
        {
            let name = "a";
            let vers = "2";
            let pack = package(name, vers, "https://rust-lang.org", "124");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);

        let ps = repo.find(&pname("a"), &pversion("2"));
        assert_eq!(ps.len(), 1);

        let p = ps.get(0).unwrap();
        assert_eq!(*p.name(), pname("a"));
        assert_eq!(*p.version(), pversion("2"));
        assert!(!p.version_is_semver());
    }

    #[test]
    fn test_find_with_vers_constr_exact() {
        let mut btree = BTreeMap::new();

        {
            let name = "a";
            let vers = "1";
            let pack = package(name, vers, "https://rust-lang.org", "123");
            btree.insert((pname(name), pversion(vers)), pack);
        }
        {
            let name = "a";
            let vers = "2";
            let pack = package(name, vers, "https://rust-lang.org", "124");
            btree.insert((pname(name), pversion(vers)), pack);
        }
        {
            let name = "a";
            let vers = "3";
            let pack = package(name, vers, "https://rust-lang.org", "125");
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);

        let constraint = PackageVersionConstraint::from_version(String::from("="), pversion("2"));

        let ps = repo.find_with_version(&pname("a"), &constraint);
        assert_eq!(ps.len(), 1);

        let p = ps.get(0).unwrap();
        assert_eq!(*p.name(), pname("a"));
        assert_eq!(*p.version(), pversion("2"));
        assert!(!p.version_is_semver());
    }
}
