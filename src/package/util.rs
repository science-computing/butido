//! Utility types for the package definitions
//!
//! These types exist only for the purpose of strong typing
//! and cannot do anything special.

use std::result::Result as RResult;
use std::ops::Deref;

use serde::Deserialize;
use anyhow::Result;

use crate::package::version::NameVersionBuffer;

#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct PackageName(String);

impl Deref for PackageName {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for PackageName {
    fn from(s: String) -> Self {
        PackageName(s)
    }
}

impl std::fmt::Display for PackageName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        self.0.fmt(f)
    }
}

#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct PackageVersion(String);

impl From<String> for PackageVersion {
    fn from(s: String) -> Self {
        PackageVersion(s)
    }
}

#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct SystemDependency(String);

impl NameVersionBuffer for SystemDependency {
    fn get_as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct BuildDependency(String);

impl NameVersionBuffer for BuildDependency {
    fn get_as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct Dependency(String);

impl NameVersionBuffer for Dependency {
    fn get_as_str(&self) -> &str {
        &self.0
    }
}

impl std::convert::TryInto<(PackageName, PackageVersionConstraint)> for Dependency {
    type Error = anyhow::Error;

    fn try_into(self) -> RResult<(PackageName, PackageVersionConstraint), Self::Error> {
        unimplemented!()
    }
}

#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
#[serde(transparent)]
pub struct HashValue(String);


/// A type which can be used to express a package version constraint
#[derive(Debug, Eq, PartialEq)]
pub enum PackageVersionConstraint {
    Any,
    Latest,
    LowerAs(PackageVersion),
    HigherAs(PackageVersion),
    InRange(PackageVersion, PackageVersion),
    Exact(PackageVersion),
}

impl PackageVersionConstraint {
    pub fn matches(&self, v: &PackageVersion) -> Result<PackageVersionMatch> {
        match self {
            PackageVersionConstraint::Any                     => Ok(PackageVersionMatch::True),
            PackageVersionConstraint::Latest                  => Ok(PackageVersionMatch::Undecided),
            PackageVersionConstraint::LowerAs(_vers)          => Ok(PackageVersionMatch::Undecided), // TODO: Fix implementation
            PackageVersionConstraint::HigherAs(_vers)         => Ok(PackageVersionMatch::Undecided), // TODO: Fix implementation
            PackageVersionConstraint::InRange(_vers1, _vers2) => Ok(PackageVersionMatch::Undecided), // TODO: Fix implementation
            PackageVersionConstraint::Exact(vers)             => Ok(PackageVersionMatch::from(*v == *vers)),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PackageVersionMatch {
    True,
    False,
    Undecided,
}

impl PackageVersionMatch {
    pub fn is_true(&self) -> bool {
        *self == PackageVersionMatch::True
    }

    pub fn is_false(&self) -> bool {
        *self == PackageVersionMatch::False
    }

    pub fn is_undecided(&self) -> bool {
        *self == PackageVersionMatch::Undecided
    }
}

impl From<bool> for PackageVersionMatch {
    fn from(b: bool) -> Self {
        if b {
            PackageVersionMatch::True
        } else {
            PackageVersionMatch::False
        }
    }
}

