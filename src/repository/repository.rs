//
// Copyright (c) 2020-2022 science+computing ag and other contributors
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
use resiter::AndThen;
use resiter::FilterMap;
use resiter::Map;
use tracing::trace;

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

        // Helper function to extract the `patches` array from a package config/definition:
        fn get_patches(
            config: &config::ConfigBuilder<config::builder::DefaultState>,
            path: &Path,
        ) -> Result<Vec<PathBuf>> {
            // TODO: Avoid unnecessary config building (inefficient):
            let config = config.build_cloned().context(anyhow!(
                "Failed to load the following TOML file: {}",
                path.display()
            ))?;
            match config.get_array("patches") {
                Ok(v) => v
                    .into_iter()
                    .map(config::Value::into_string)
                    .map_err(Error::from)
                    .map_err(|e| e.context("patches must be strings"))
                    .map_err(Error::from)
                    .map_ok(PathBuf::from)
                    .collect(),
                Err(config::ConfigError::NotFound(_)) => Ok(Vec::with_capacity(0)),
                Err(e) => Err(Error::from(e)),
            }
        }

        let leaf_files = fsr
            .files()
            .par_iter()
            .inspect(|path| trace!("Checking for leaf file: {}", path.display()))
            .filter_map(|path| match fsr.is_leaf_file(path) {
                Ok(true) => Some(Ok(path)),
                Ok(false) => None,
                Err(e) => Some(Err(e)),
            });
        progress.set_length(leaf_files.clone().count().try_into()?);
        leaf_files.inspect(|r| trace!("Loading files for {:?}", r))
            .map(|path| {
                progress.inc(1);
                let path = path?;
                fsr.get_files_for(path)?
                    .iter()
                    .inspect(|(path, _)| trace!("Loading layer at {}", path.display()))
                    .fold(Ok(Config::builder()) as Result<_>, |config, (path, content)| {
                        let mut config = config?;

                        let patches_before_merge = get_patches(&config, path)?;
                        config = config.add_source(config::File::from_str(content, config::FileFormat::Toml));
                        let patches_after_merge = get_patches(&config, path)?;

                        // TODO: Get rid of the unnecessarily complex handling of the `patches` configuration setting:
                        // Ideally this would be handled by the `config` crate (this is
                        // already the case for all other "settings" but in this case we also need
                        // to prepend the corresponding directory path).
                        let patches = if patches_before_merge == patches_after_merge {
                            patches_before_merge
                        } else {
                            // The patches have changed since the `config.merge()` of the next
                            // `pkg.toml` file so we have to build the paths to the patch files
                            // by prepending the path to the directory of the `pkg.toml` file since
                            // `path` is only available in this "iteration".
                            patches_after_merge
                                .into_iter()
                                // Prepend the path of the directory of the `pkg.toml` file to the name of the patch:
                                .map(|p| if let Some(current_dir) = path.parent() {
                                    Ok(current_dir.join(p))
                                } else {
                                    Err(anyhow!("Path should point to path with parent, but doesn't: {}", path.display()))
                                })
                                .inspect(|patch| trace!("Patch: {:?}", patch))
                                // If the patch file exists, use it (as config::Value).
                                // Otherwise we have an error here, because we're referring to a non-existing file:
                                .and_then_ok(|patch| if patch.exists() {
                                    Ok(Some(patch))
                                } else {
                                    Err(anyhow!("Patch does not exist: {}", patch.display()))
                                        .with_context(|| anyhow!("The patch is declared here: {}", path.display()))
                                })
                                .filter_map_ok(|o| o)
                                .collect::<Result<Vec<_>>>()?
                        };

                        trace!("Patches after postprocessing merge: {:?}", patches);
                        let patches = patches
                            .into_iter()
                            .map(|p| p.display().to_string())
                            .map(config::Value::from)
                            .collect::<Vec<_>>();
                        {
                            // Update the `patches` configuration setting:
                            let mut builder = Config::builder();
                            builder = builder.set_default("patches", config::Value::from(patches))?;
                            config = config.add_source(builder.build()?);
                            // Ideally we'd use `config.set()` but that is a permanent override (so
                            // subsequent `config.merge()` merges won't have an effect on
                            // "patches"). There's also `config.set_once()` but that only lasts
                            // until the next `config.merge()` and `config.set_default()` only sets
                            // a default value.
                        }
                        Ok(config)
                    })
                    .and_then(|c| c.build()?.try_deserialize::<Package>().map_err(Error::from)
                        .with_context(|| anyhow!("Could not load package configuration: {}", path.display())))
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

        let p = ps.first().unwrap();
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

        let p = ps.first().unwrap();
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

        let p = ps.first().unwrap();
        assert_eq!(*p.name(), pname("a"));
        assert_eq!(*p.version(), pversion("2"));
        assert!(!p.version_is_semver());
    }

    #[test]
    fn test_load_example_pkg_repo() -> Result<()> {
        use crate::package::Package;

        fn get_pkg(repo: &Repository, name: &str, version: &str) -> Package {
            let constraint =
                PackageVersionConstraint::from_version(String::from("="), pversion(version));
            let pkgs = repo.find_with_version(&pname(name), &constraint);
            assert_eq!(pkgs.len(), 1, "Failed to find pkg: {name} ={version}");
            (*pkgs.first().unwrap()).clone()
        }
        fn assert_pkg(repo: &Repository, name: &str, version: &str) {
            let p = get_pkg(repo, name, version);
            assert_eq!(*p.name(), pname(name));
            assert_eq!(*p.version(), pversion(version));
            assert_eq!(p.sources().len(), 1);
        }

        let repo = Repository::load(
            &PathBuf::from("examples/packages/repo/"),
            &indicatif::ProgressBar::hidden(),
        )?;

        assert_pkg(&repo, "a", "1");
        assert_pkg(&repo, "b", "2");
        assert_pkg(&repo, "c", "3");
        assert_pkg(&repo, "s", "19.0");
        assert_pkg(&repo, "s", "19.1");
        assert_pkg(&repo, "z", "26");

        // Verify the paths of the patches (and "merging"):
        // The patches are defined as follows:
        // s/pkg.toml: patches = [ "./foo.patch" ]
        // s/19.0/pkg.toml: patches = ["./foo.patch","s190.patch"]
        // s/19.1/pkg.toml: - (no `patches` entry)
        // s/19.2/pkg.toml: patches = ["../foo.patch"]
        // s/19.3/pkg.toml: patches = ["s190.patch"]
        let p = get_pkg(&repo, "s", "19.0");
        // Ideally we'd normalize the `./` away:
        assert_eq!(
            p.patches(),
            &vec![
                PathBuf::from("examples/packages/repo/s/19.0/./foo.patch"),
                PathBuf::from("examples/packages/repo/s/19.0/s190.patch")
            ]
        );
        let p = get_pkg(&repo, "s", "19.1");
        assert_eq!(
            p.patches(),
            &vec![PathBuf::from("examples/packages/repo/s/foo.patch")]
        );
        let p = get_pkg(&repo, "s", "19.2");
        // We might want to normalize the `19.2/../` away:
        assert_eq!(
            p.patches(),
            &vec![PathBuf::from("examples/packages/repo/s/19.2/../foo.patch")]
        );
        let p = get_pkg(&repo, "s", "19.3");
        assert_eq!(
            p.patches(),
            &vec![PathBuf::from("examples/packages/repo/s/19.3/s193.patch")]
        );

        Ok(())
    }
}
