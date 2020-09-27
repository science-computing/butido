//! Utility types for the package definitions
//!
//! These types exist only for the purpose of strong typing
//! and cannot do anything special.

use serde::Deserialize;

#[derive(Deserialize, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct PackageName(String);

#[derive(Deserialize, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct PackageVersion(String);

#[derive(Deserialize, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct SystemDependency(String);

#[derive(Deserialize, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct BuildDependency(String);

#[derive(Deserialize, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Dependency(String);

#[derive(Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct HashValue(String);

