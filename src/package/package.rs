use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use getset::Getters;
use itertools::Itertools;
use serde::Deserialize;
use serde::Serialize;

use crate::package::ParseDependency;
use crate::package::dependency::*;
use crate::package::name::*;
use crate::package::source::*;
use crate::package::version::*;
use crate::phase::{PhaseName, Phase};
use crate::util::EnvironmentVariableName;
use crate::util::docker::ImageName;

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
    flags: Option<PackageFlags>,

    #[getset(get = "pub")]
    #[serde(skip_serializing_if = "Option::is_none")]
    deny_on_images: Option<Vec<ImageName>>,

    #[getset(get = "pub")]
    phases: HashMap<PhaseName, Phase>,
}

impl Package {

    #[cfg(test)]
    pub fn new(name: PackageName, version: PackageVersion, version_is_semver: bool, sources: HashMap<String, Source>, dependencies: Dependencies) -> Self {
        Package {
            name,
            version,
            version_is_semver,
            sources,
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

    pub fn get_self_packaged_dependencies<'a>(&'a self) -> impl Iterator<Item = Result<(PackageName, PackageVersionConstraint)>> + 'a {
        let build_iter = self.dependencies()
            .build()
            .iter()
            .cloned()
            .map(|d| d.parse_as_name_and_version());

        let runtime_iter = self.dependencies()
            .runtime()
            .iter()
            .cloned()
            .map(|d| d.parse_as_name_and_version());

        build_iter
            .chain(runtime_iter)
            .unique_by(|res| res.as_ref().ok().map(|tpl| tpl.clone()))
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


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageFlags {
    build_parallel: bool,
}

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
        let sources = {
            let url       = Url::parse(srcurl).unwrap();
            let hashvalue = HashValue::from(String::from(hash));
            let mut hm = HashMap::new();
            hm.insert(String::from("src"), Source::new(url, SourceHash::new(HashType::Sha1, hashvalue)));
            hm
        };
        let dependencies = Dependencies::empty();
        Package::new(name, version, version_is_semver, sources, dependencies)
    }

}
