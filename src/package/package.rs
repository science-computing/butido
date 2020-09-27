use std::path::PathBuf;
use std::collections::HashMap;

use url::Url;
use getset::Getters;
use serde::Deserialize;

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

