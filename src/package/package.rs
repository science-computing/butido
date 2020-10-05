use std::path::PathBuf;
use std::collections::HashMap;

use url::Url;
use getset::Getters;
use serde::Deserialize;
use anyhow::Result;

use crate::phase::{PhaseName, Phase};
use crate::package::util::*;
use crate::util::docker::ImageName;

#[derive(Debug, Deserialize, Getters)]
pub struct Package {
    #[getset(get = "pub")]
    name: PackageName,

    #[getset(get = "pub")]
    version: PackageVersion,

    #[getset(get = "pub")]
    version_is_semver: bool,

    #[getset(get = "pub")]
    source_url: Url,

    #[getset(get = "pub")]
    source_hash: SourceHash,

    #[getset(get = "pub")]
    system_dependencies: Vec<SystemDependency>,

    #[getset(get = "pub")]
    #[serde(skip_serializing_if = "Option::is_none")]
    system_dependencies_script: Option<PathBuf>,

    #[getset(get = "pub")]
    build_dependencies: Vec<BuildDependency>,

    #[getset(get = "pub")]
    #[serde(skip_serializing_if = "Option::is_none")]
    build_dependencies_script: Option<PathBuf>,

    #[getset(get = "pub")]
    dependencies: Vec<Dependency>,

    #[getset(get = "pub")]
    #[serde(skip_serializing_if = "Option::is_none")]
    dependencies_script: Option<PathBuf>,

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
    #[serde(skip_serializing_if = "Option::is_none")]
    phases: Option<HashMap<PhaseName, Phase>>,
}

impl Package {
    /// Get all dependencies of the package
    ///
    /// Either return the list of dependencies or, if available, run the dependencies_script to
    /// read the dependencies from there.
    pub fn get_all_dependencies(&self) -> Result<Vec<(PackageName, PackageVersionConstraint)>> {
        use std::convert::TryInto;

        // TODO: Current implementation does not run dependency script
        //

        self.dependencies.iter().map(|d| d.clone().try_into()).collect()
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

#[derive(Debug, Deserialize)]
pub struct SourceHash {
    #[serde(rename = "type")]
    hashtype: HashType,

    #[serde(rename = "hash")]
    value: HashValue,
}

#[derive(Debug, Deserialize)]
pub enum HashType {
    #[serde(rename = "sha1")]
    Sha1,

    #[serde(rename = "sha256")]
    Sha256,

    #[serde(rename = "sha512")]
    Sha512,
}

#[derive(Debug, Deserialize)]
pub struct PackageFlags {
    build_parallel: bool,
}

