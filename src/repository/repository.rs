use std::collections::BTreeMap;
use std::path::PathBuf;
use std::path::Path;
use anyhow::Result;
use anyhow::Context;
use resiter::Map;

use crate::package::Package;
use crate::package::PackageName;
use crate::package::PackageVersion;
use crate::package::PackageVersionConstraint;

/// A repository represents a collection of packages
pub struct Repository {
    inner: BTreeMap<(PackageName, PackageVersion), Package>,
}

impl Repository {

    pub fn load(path: &Path, progress: &indicatif::ProgressBar) -> Result<Self> {
        fn all_subdirs(p: &Path) -> Result<Vec<PathBuf>> {
            let mut v = Vec::new();
            for de in p.read_dir()? {
                let de = de?;
                if de.file_type()?.is_dir() {
                    v.push(de.path());
                }
            }

            return Ok(v)
        }

        fn load_recursive(path: &Path, mut config: config::Config, progress: &indicatif::ProgressBar) -> Result<Vec<Result<Package>>> {
            let pkg_file = path.join("pkg.toml");

            if pkg_file.is_file() {
                let buf = std::fs::read_to_string(&pkg_file)
                    .with_context(|| format!("Reading {}", pkg_file.display()))?;

                config.merge(config::File::from_str(&buf, config::FileFormat::Toml))
                    .with_context(|| format!("Loading contents of {}", pkg_file.display()))?;
            }

            let subdirs = all_subdirs(path).with_context(|| format!("Finding subdirs for {}", pkg_file.display()))?;

            if subdirs.is_empty() {
                let package = config
                    .deserialize()
                    .with_context(|| format!("Failed to parse {} into package", path.display()));

                progress.tick();
                Ok(vec![package])
            } else {
                subdirs
                    .into_iter()
                    .fold(Ok(Vec::new()), |vec, dir| {
                        vec.and_then(|mut v| {
                            let mut loaded = load_recursive(&dir, config.clone(), progress)
                                .with_context(|| format!("Recursing for {}", pkg_file.display()))?;

                            v.append(&mut loaded);
                            Ok(v)
                        })
                    })
            }
        }

        let inner = load_recursive(path, config::Config::default(), progress)
            .with_context(|| format!("Recursing for {}", path.display()))?
            .into_iter()
            .map_ok(|p| ((p.name().clone(), p.version().clone()), p))
            .collect::<Result<_>>()?;

        Ok(Repository { inner })
    }

    pub fn find_by_name<'a>(&'a self, name: &PackageName) -> Vec<&'a Package> {
        self.inner
            .iter()
            .filter(|((n, _), _)| name == n)
            .map(|(_, pack)| pack)
            .collect()
    }

    pub fn find<'a>(&'a self, name: &PackageName, version: &PackageVersion) -> Option<&'a Package> {
        self.inner
            .iter()
            .find(|((n, v), _)| n == name && v == version)
            .map(|(_, p)| p)
    }

    pub fn find_with_version_contraint<'a>(&'a self, name: &PackageName, vc: &PackageVersionConstraint) -> Vec<&'a Package> {
        self.inner
            .iter()
            .filter(|((n, v), _)| {
                n == name && vc.matches(v).map(|mtch| !mtch.is_false()).unwrap_or(false)
            })
            .map(|(_, p)| p)
            .collect()
    }
}
