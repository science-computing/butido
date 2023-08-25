//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::collections::HashMap;
use std::path::PathBuf;

use getset::Getters;
use serde::Deserialize;
use serde::Serialize;

use crate::package::dependency::*;
use crate::package::name::*;
use crate::package::source::*;
use crate::package::version::*;
use crate::package::{Phase, PhaseName};
use crate::util::docker::ImageName;
use crate::util::EnvironmentVariableName;

#[derive(Clone, Serialize, Deserialize, Getters)]
pub struct Package {
    #[getset(get = "pub")]
    name: PackageName,

    #[getset(get = "pub")]
    version: PackageVersion,

    #[getset(get = "pub")]
    version_is_semver: bool,

    #[getset(get = "pub")]
    sources: HashMap<String, Source>,

    #[getset(get = "pub")]
    dependencies: Dependencies,

    #[getset(get = "pub")]
    patches: Vec<PathBuf>,

    #[getset(get = "pub")]
    #[serde(skip_serializing_if = "Option::is_none")]
    environment: Option<HashMap<EnvironmentVariableName, String>>,

    #[getset(get = "pub")]
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_images: Option<Vec<ImageName>>,

    #[getset(get = "pub")]
    #[serde(skip_serializing_if = "Option::is_none")]
    denied_images: Option<Vec<ImageName>>,

    #[getset(get = "pub")]
    phases: HashMap<PhaseName, Phase>,

    /// Meta field
    ///
    /// Contains only key-value string-string data, that the packager can set for a package and
    /// then use in the packaging scripts (for example) to write package meta data to the package
    /// file (think of rpmbuild spec files).
    #[getset(get = "pub")]
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<HashMap<String, String>>,
}

impl std::hash::Hash for Package {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.version.hash(state);
    }
}

impl Package {
    #[cfg(test)]
    pub fn new(
        name: PackageName,
        version: PackageVersion,
        version_is_semver: bool,
        sources: HashMap<String, Source>,
        dependencies: Dependencies,
    ) -> Self {
        Package {
            name,
            version,
            version_is_semver,
            sources,
            dependencies,
            patches: vec![],
            environment: None,
            allowed_images: None,
            denied_images: None,
            phases: HashMap::new(),
            meta: None,
        }
    }

    #[cfg(test)]
    pub fn set_dependencies(&mut self, dependencies: Dependencies) {
        self.dependencies = dependencies;
    }

    /// Get a wrapper object around self which implements a debug interface with all details about
    /// the Package object
    #[cfg(debug_assertions)]
    pub fn debug_details(&self) -> DebugPackage<'_> {
        DebugPackage(self)
    }
}

impl std::fmt::Debug for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        if self.patches().is_empty() {
            write!(f, "Package({:?}, {:?})", self.name(), self.version())
        } else {
            write!(
                f,
                "Package({:?}, {:?} + patches)",
                self.name(),
                self.version()
            )
        }
    }
}

/// Helper type for printing debug information about a package with much more details than the
/// Debug impl for Package provides.
#[cfg(debug_assertions)]
pub struct DebugPackage<'a>(&'a Package);

#[cfg(debug_assertions)]
impl<'a> std::fmt::Debug for DebugPackage<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        writeln!(
            f,
            "Package({name} {version} ({semver}))",
            name = self.0.name,
            version = self.0.version,
            semver = if self.0.version_is_semver {
                "is semver"
            } else {
                "not semver"
            }
        )?;

        writeln!(f, "\tSources = ")?;
        self.0.sources.iter().try_for_each(|(k, v)| {
            writeln!(
                f,
                "\t\t{name} = (Url = {url}, Hash = {hash} ({hasht}), {dl})",
                name = k,
                url = v.url(),
                hash = v.hash().value(),
                hasht = v.hash().hashtype(),
                dl = if *v.download_manually() {
                    "manual download"
                } else {
                    "automatic download"
                },
            )
        })?;

        writeln!(f, "\tBuild Dependencies = ")?;
        self.0
            .dependencies
            .build
            .iter()
            .try_for_each(|d| writeln!(f, "\t\t{d:?}"))?;

        writeln!(f, "\tRuntime Dependencies = ")?;
        self.0
            .dependencies
            .runtime
            .iter()
            .try_for_each(|r| writeln!(f, "\t\t{r:?}"))?;

        writeln!(f, "\tPatches = ")?;
        self.0
            .patches
            .iter()
            .try_for_each(|p| writeln!(f, "\t\t{}", p.display()))?;

        writeln!(f, "\tEnvironment = ")?;
        self.0
            .environment
            .as_ref()
            .map(|hm| {
                hm.iter()
                    .try_for_each(|(k, v)| writeln!(f, "\t\t{k:?} = {v}"))
            })
            .transpose()?;

        writeln!(f, "\tAllowed Images = ")?;

        self.0
            .allowed_images
            .as_ref()
            .map(|v| v.iter().try_for_each(|i| writeln!(f, "\t\t{i:?}")))
            .transpose()?;

        writeln!(f, "\tDenied Images = ")?;
        self.0
            .denied_images
            .as_ref()
            .map(|v| v.iter().try_for_each(|i| writeln!(f, "\t\t{i:?}")))
            .transpose()?;

        writeln!(f, "\tPhases = ")?;
        self.0
            .phases
            .iter()
            .try_for_each(|(k, _)| writeln!(f, "\t\t{k:?} = ..."))?;

        Ok(())
    }
}

impl PartialEq for Package {
    fn eq(&self, other: &Package) -> bool {
        (self.name(), self.version()).eq(&(other.name(), other.version()))
    }
}

impl PartialOrd for Package {
    fn partial_cmp(&self, other: &Package) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Package {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.name(), self.version()).cmp(&(other.name(), other.version()))
    }
}

impl Eq for Package {}

#[derive(Clone, Debug, Serialize, Deserialize, Getters)]
pub struct Dependencies {
    #[getset(get = "pub")]
    build: Vec<BuildDependency>,

    #[getset(get = "pub")]
    runtime: Vec<Dependency>,
}

#[cfg(test)]
impl Dependencies {
    pub fn empty() -> Self {
        Dependencies {
            build: vec![],
            runtime: vec![],
        }
    }

    pub fn with_runtime_dependency(runtime_dependency: Dependency) -> Self {
        Dependencies::with_runtime_dependencies(vec![runtime_dependency])
    }

    pub fn with_runtime_dependencies(runtime_dependencies: Vec<Dependency>) -> Self {
        Dependencies {
            build: vec![],
            runtime: runtime_dependencies,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::package::Dependencies;
    use crate::package::HashType;
    use crate::package::HashValue;
    use crate::package::Source;
    use crate::package::SourceHash;
    use url::Url;

    /// helper function for quick object construction
    pub fn pname(name: &str) -> PackageName {
        PackageName::from(String::from(name))
    }

    /// helper function for quick object construction
    pub fn pversion(version: &str) -> PackageVersion {
        PackageVersion::from(String::from(version))
    }

    /// helper function for quick object construction
    pub fn package(name: &str, vers: &str, srcurl: &str, hash: &str) -> Package {
        let name = pname(name);
        let version = pversion(vers);
        let version_is_semver = false;
        let sources = {
            let url = Url::parse(srcurl).unwrap();
            let hashvalue = HashValue::from(String::from(hash));
            let mut hm = HashMap::new();
            hm.insert(
                String::from("src"),
                Source::new(url, SourceHash::new(HashType::Sha1, hashvalue)),
            );
            hm
        };
        let dependencies = Dependencies::empty();
        Package::new(name, version, version_is_semver, sources, dependencies)
    }
}
