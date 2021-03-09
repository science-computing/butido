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

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use log::trace;
use resiter::Map;
use resiter::FilterMap;

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
    pub fn load(path: &Path, progress: &indicatif::ProgressBar) -> Result<Self> {
        fn all_subdirs(p: &Path) -> Result<Vec<PathBuf>> {
            let mut v = Vec::new();
            for de in p.read_dir()? {
                let de = de?;
                let is_dir = de.file_type()?.is_dir();
                let is_hidden = de
                    .path()
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.starts_with('.'))
                    .unwrap_or(false);

                if is_dir && !is_hidden {
                    v.push(de.path());
                }
            }

            Ok(v)
        }

        fn load_recursive(
            root: &Path,
            path: &Path,
            mut config: config::Config,
            progress: &indicatif::ProgressBar,
        ) -> Result<Vec<Result<Package>>> {
            let pkg_file = path.join("pkg.toml");

            if pkg_file.is_file() {
                let buf = std::fs::read_to_string(&pkg_file)
                    .with_context(|| format!("Reading {}", pkg_file.display()))?;

                // This function has an issue: It loads packages recursively, but if there are
                // patches set for a package, these patches are set _relative_ to the current
                // pkg.toml file.
                //
                // E.G.:
                // (1) /pkg.toml
                // (2) /a/pkg.toml
                // (3) /a/1.0/pkg.toml
                // (4) /a/2.0/pkg.toml
                //
                // If (2) defines a patches = ["./1.patch"], the patch exists at /a/1.patch.
                // We can fix that by modifying the Config object after loading (2) and fixing the
                // path of the patch to be relative to the repostory root.
                //
                // But if we continue loading the /a/ subdirectory recursively, this value gets
                // overwritten by Config::refresh(), which is called by Config::merge, for example.
                //
                // The trick is, to get the list of patches _before_ the merge, and later
                // re-setting them after the merge, if there were no new patches set (which itself
                // is tricky to find out, because the `Config` object _looks like_ there is a new
                // array set).
                //
                // If (3), for example, does set a new patches=[] array, the old array is
                // invalidated and no longer relevant for that package!
                // Thus, we can savely throw it away and continue with the new array, fixing the
                // pathes to be relative to repo root again.
                //
                // If (4) does _not_ set any patches, we must ensure that the patches from the
                // loading of (2) are used and not overwritten by the Config::refresh() call
                // happening during Config::merge().
                //

                let patches_before_merge = match config.get_array("patches") {
                    Ok(v)                                 => v,
                    Err(config::ConfigError::NotFound(_)) => vec![],
                    Err(e)                                => return Err(e).map_err(Error::from),
                };
                trace!("Patches before merging: {:?}", patches_before_merge);

                // Merge the new pkg.toml file over the already loaded configuration
                config
                    .merge(config::File::from_str(&buf, config::FileFormat::Toml))
                    .with_context(|| format!("Loading contents of {}", pkg_file.display()))?;

                let patches_after_merge = match config.get_array("patches") {
                    Ok(v)                                 => v,
                    Err(config::ConfigError::NotFound(_)) => vec![],
                    Err(e)                                => return Err(e).map_err(Error::from),
                };
                trace!("Patches after merging: {:?}", patches_after_merge);

                let path_relative_to_root = path.strip_prefix(root)?;
                let patches_after_merge = patches_after_merge
                    .into_iter()
                    .map(|patch| {
                        let patch = patch.into_str()?;
                        let new_path = path_relative_to_root.join(&patch);
                        if new_path.exists() {
                            Ok(Some(new_path))
                        } else {
                            Ok(None)
                        }
                    })
                    .filter_map_ok(|x| x)
                    .map_ok(|p| config::Value::from(p.display().to_string()))
                    .collect::<Result<Vec<_>>>()?;

                let patches = if patches_after_merge.is_empty() {
                    patches_before_merge
                } else {
                    patches_after_merge
                };

                trace!("Patches after postprocessing merge: {:?}", patches);
                config.set_once("patches", config::Value::from(patches))?;
            }

            let subdirs = all_subdirs(path)
                .with_context(|| format!("Finding subdirs for {}", pkg_file.display()))?;

            if subdirs.is_empty() {
                progress.tick();
                if pkg_file.is_file() {
                    let package = config.try_into().with_context(|| {
                        format!("Failed to parse {} into package", path.display())
                    });

                    Ok(vec![package])
                } else {
                    Ok(vec![])
                }
            } else {
                subdirs.into_iter().fold(Ok(Vec::new()), |vec, dir| {
                    vec.and_then(|mut v| {
                        trace!("Recursing into {}", dir.display());
                        let mut loaded = load_recursive(root, &dir, config.clone(), progress)
                            .with_context(|| format!("Recursing for {}", pkg_file.display()))?;

                        v.append(&mut loaded);
                        Ok(v)
                    })
                })
            }
        }

        let inner = load_recursive(path, path, config::Config::default(), progress)
            .with_context(|| format!("Recursing for {}", path.display()))?
            .into_iter()
            .inspect(|p| trace!("Loading into repository: {:?}", p))
            .map_ok(|p| ((p.name().clone(), p.version().clone()), p))
            .collect::<Result<_>>()?;

        Ok(Repository { inner })
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
