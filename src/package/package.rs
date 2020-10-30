use std::path::PathBuf;
use std::collections::HashMap;

use getset::Getters;
use serde::Deserialize;
use anyhow::Result;
use anyhow::Error;
use resiter::AndThen;

use crate::phase::{PhaseName, Phase};
use crate::package::dependency::*;
use crate::package::source::*;
use crate::package::name::*;
use crate::package::version::*;
use crate::util::docker::ImageName;
use crate::package::ParseDependency;

#[derive(Clone, Deserialize, Getters)]
pub struct Package {
    #[getset(get = "pub")]
    name: PackageName,

    #[getset(get = "pub")]
    version: PackageVersion,

    #[getset(get = "pub")]
    version_is_semver: bool,

    #[getset(get = "pub")]
    source: Source,

    #[getset(get = "pub")]
    dependencies: Dependencies,

    #[getset(get = "pub")]
    patches: Vec<PathBuf>,

    #[getset(get = "pub")]
    #[serde(skip_serializing_if = "Option::is_none")]
    environment: Option<HashMap<String, String>>,

    #[getset(get = "pub")]
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<PackageFlags>,

    #[getset(get = "pub")]
    #[serde(skip_serializing_if = "Option::is_none")]
    deny_on_images: Option<Vec<ImageName>>,

    #[getset(get = "pub")]
    phases: HashMap<PhaseName, Phase>,
}

impl Package {

    #[cfg(test)]
    pub fn new(name: PackageName, version: PackageVersion, version_is_semver: bool, source: Source, dependencies: Dependencies) -> Self {
        Package {
            name,
            version,
            version_is_semver,
            source,
            dependencies,
            patches: vec![],
            environment: None,
            flags: None,
            deny_on_images: None,
            phases: HashMap::new(),
        }
    }

    #[cfg(test)]
    pub fn set_dependencies(&mut self, dependencies: Dependencies) {
        self.dependencies = dependencies;
    }

    /// Get all dependencies of the package
    ///
    /// Either return the list of dependencies or, if available, run the dependencies_script to
    /// read the dependencies from there.
    pub fn get_all_dependencies(&self) -> Result<Vec<(PackageName, PackageVersionConstraint)>> {
        self.get_system_dependencies()?
            .into_iter()
            .map(Ok)
            .chain(self.get_self_packaged_dependencies()?.into_iter().map(Ok))
            .collect()
    }

    pub fn get_system_dependencies(&self) -> Result<Vec<(PackageName, PackageVersionConstraint)>> {
        let system_iter = self.dependencies()
            .system()
            .iter()
            .cloned()
            .map(|d| d.parse_into_name_and_version());

        let system_runtime_iter = self.dependencies()
            .system_runtime()
            .iter()
            .cloned()
            .map(|d| d.parse_into_name_and_version());

        system_iter.chain(system_runtime_iter).collect()
    }

    pub fn get_self_packaged_dependencies(&self) -> Result<Vec<(PackageName, PackageVersionConstraint)>> {
        let build_iter = self.dependencies()
            .build()
            .iter()
            .cloned()
            .map(|d| d.parse_into_name_and_version());

        let runtime_iter = self.dependencies()
            .runtime()
            .iter()
            .cloned()
            .map(|d| d.parse_into_name_and_version());

        build_iter.chain(runtime_iter).collect()
    }
}

impl std::fmt::Debug for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        if self.patches().is_empty() {
            write!(f, "Package({:?}, {:?})", self.name(), self.version())
        } else {
            write!(f, "Package({:?}, {:?} + patches)", self.name(), self.version())
        }
    }
}

impl PartialEq for Package {
    fn eq(&self, other: &Package) -> bool {
        (self.name(), self.version()).eq(&(other.name(), other.version()))
    }
}

impl PartialOrd for Package {
    fn partial_cmp(&self, other: &Package) -> Option<std::cmp::Ordering> {
        (self.name(), self.version()).partial_cmp(&(other.name(), other.version()))
    }
}

impl Ord for Package {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.name(), self.version()).cmp(&(other.name(), other.version()))
    }
}

impl Eq for Package {
}


#[derive(Clone, Debug, Deserialize)]
pub struct PackageFlags {
    build_parallel: bool,
}

#[derive(Clone, Debug, Deserialize, Getters)]
pub struct Dependencies {
    #[getset(get = "pub")]
    system: Vec<SystemBuildDependency>,

    #[getset(get = "pub")]
    system_runtime: Vec<SystemDependency>,

    #[getset(get = "pub")]
    build: Vec<BuildDependency>,

    #[getset(get = "pub")]
    runtime: Vec<Dependency>,
}

#[cfg(test)]
impl Dependencies {
    pub fn empty() -> Self {
        Dependencies {
            system: vec![],
            system_runtime: vec![],
            build: vec![],
            runtime: vec![],
        }
    }

    pub fn with_runtime_dependency(runtime_dependency: Dependency) -> Self {
        Dependencies::with_runtime_dependencies(vec![runtime_dependency])
    }

    pub fn with_runtime_dependencies(runtime_dependencies: Vec<Dependency>) -> Self {
        Dependencies {
            system: vec![],
            system_runtime: vec![],
            build: vec![],
            runtime: runtime_dependencies,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use url::Url;
    use crate::package::Source;
    use crate::package::SourceHash;
    use crate::package::HashType;
    use crate::package::HashValue;
    use crate::package::Dependencies;

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
        let name    = pname(name);
        let version = pversion(vers);
        let version_is_semver = false;
        let source = {
            let url       = Url::parse(srcurl).unwrap();
            let hashvalue = HashValue::from(String::from(hash));
            Source::new(url, SourceHash::new(HashType::Sha1, hashvalue))
        };
        let dependencies = Dependencies::empty();
        Package::new(name, version, version_is_semver, source, dependencies)
    }

}
