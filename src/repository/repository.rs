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
use std::path::Component;
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

// A helper function to normalize relative Unix paths (ensures that one cannot escape using `..`):
fn normalize_relative_path(path: PathBuf) -> Result<PathBuf> {
    let mut normalized_path = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) => {
                // "A Windows path prefix, e.g., C: or \\server\share."
                // "Does not occur on Unix."
                return Err(anyhow!(
                    "The relative path \"{}\" starts with a Windows path prefix",
                    path.display()
                ));
            }
            Component::RootDir => {
                // "The root directory component, appears after any prefix and before anything else.
                // It represents a separator that designates that a path starts from root."
                return Err(anyhow!(
                    "The relative path \"{}\" starts from the root directory",
                    path.display()
                ));
            }
            Component::CurDir => {
                // "A reference to the current directory, i.e., `.`."
                // Also (from `Path.components()`): "Occurrences of . are normalized away, except
                // if they are at the beginning of the path. For example, a/./b, a/b/, a/b/. and
                // a/b all have a and b as components, but ./a/b starts with an additional CurDir
                // component."
                // -> May only occur as the first path component and we can ignore it / normalize
                // it away (we should just ensure that it's not the only path component in which
                // case the path would be empty).
            }
            Component::ParentDir => {
                // "A reference to the parent directory, i.e., `..`."
                if !normalized_path.pop() {
                    return Err(anyhow!(
                        "The relative path \"{}\" uses `..` to escape the base directory",
                        path.display()
                    ));
                }
            }
            Component::Normal(component) => {
                // "A normal component, e.g., a and b in a/b. This variant is the most common one,
                // it represents references to files or directories."
                normalized_path.push(component);
            }
        }
    }

    if normalized_path.parent().is_none() {
        // Optional: Convert "" to ".":
        normalized_path.push(Component::CurDir);
    }

    Ok(normalized_path)
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

        let cwd = std::env::current_dir()?;
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
                                .map(|p| if let Some(current_dir) = path.parent() {
                                    // Prepend the path of the directory of the `pkg.toml` file to
                                    // the name of the patch file:
                                    let mut path = current_dir.join(p);
                                    // Ensure that we use relative paths for the patches (the rest
                                    // of the code that uses the patches doesn't work correctly
                                    // with absolute paths):
                                    if path.is_absolute() {
                                        // We assume that cwd is part of the prefix (currently, the
                                        // path comes from `git2::Repository::workdir()` and should
                                        // always be absolute and start from cwd so this is fine):
                                        path = path
                                            .strip_prefix(&cwd)
                                            .map(|p| p.to_owned())
                                            .with_context(|| anyhow!("Cannot strip the prefix {} form path {}", cwd.display(), current_dir.display()))?;
                                    }
                                    if path.is_absolute() {
                                        Err(anyhow!("The path {} is absolute but it should be a relative path.", path.display()))
                                    } else {
                                        normalize_relative_path(path)
                                    }
                                } else {
                                    Err(anyhow!("Path should point to path with parent, but doesn't: {}", path.display()))
                                })
                                .inspect(|patch| trace!("Patch: {:?}", patch))
                                // If the patch file exists, use it (as config::Value).
                                // Otherwise we have an error here, because we're referring to a non-existing file:
                                .and_then_ok(|patch| if patch.exists() {
                                    Ok(Some(patch))
                                } else {
                                    Err(anyhow!("The following patch does not exist: {}", patch.display()))
                                })
                                .filter_map_ok(|o| o)
                                .collect::<Result<Vec<_>>>()
                                .with_context(|| anyhow!("Could not process the patches declared here: {}", path.display()))?
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
        version: &PackageVersion,
    ) -> Vec<&'a Package> {
        self.inner
            .iter()
            .filter(|((n, v), _)| n == name && v == version)
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

        let version = pversion("=2");

        let ps = repo.find_with_version(&pname("a"), &version);
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
            let version = pversion(version);
            let pkgs = repo.find_with_version(&pname(name), &version);
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

        // Verify the paths of the patches (and the base directory "merging"/joining logic plus the
        // normalization of relative paths):
        // The patches are defined as follows:
        // s/pkg.toml: patches = [ "./foo.patch" ]
        // s/19.0/pkg.toml: patches = ["./foo.patch","s190.patch"]
        // s/19.1/pkg.toml: - (no `patches` entry)
        // s/19.2/pkg.toml: patches = ["../foo.patch"]
        // s/19.3/pkg.toml: patches = ["s190.patch"]
        let p = get_pkg(&repo, "s", "19.0");
        assert_eq!(
            p.patches(),
            &vec![
                PathBuf::from("examples/packages/repo/s/19.0/foo.patch"),
                PathBuf::from("examples/packages/repo/s/19.0/s190.patch")
            ]
        );
        let p = get_pkg(&repo, "s", "19.1");
        assert_eq!(
            p.patches(),
            &vec![PathBuf::from("examples/packages/repo/s/foo.patch")]
        );
        let p = get_pkg(&repo, "s", "19.2");
        assert_eq!(
            p.patches(),
            &vec![PathBuf::from("examples/packages/repo/s/foo.patch")]
        );
        let p = get_pkg(&repo, "s", "19.3");
        assert_eq!(
            p.patches(),
            &vec![PathBuf::from("examples/packages/repo/s/19.3/s193.patch")]
        );

        Ok(())
    }

    #[test]
    fn test_relative_path_normalization() -> Result<()> {
        assert!(normalize_relative_path(PathBuf::from("/root")).is_err());
        assert!(normalize_relative_path(PathBuf::from("a/../../root")).is_err());
        assert_eq!(
            normalize_relative_path(PathBuf::from(""))?,
            PathBuf::from(".")
        );
        assert_eq!(
            normalize_relative_path(PathBuf::from("."))?,
            PathBuf::from(".")
        );
        assert_eq!(
            normalize_relative_path(PathBuf::from("./a//b/../b/./c/."))?,
            PathBuf::from("a/b/c")
        );
        assert_eq!(
            normalize_relative_path(PathBuf::from("./a//../b/"))?,
            PathBuf::from("b")
        );

        Ok(())
    }
}
