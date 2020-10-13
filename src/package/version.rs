use serde::Deserialize;
use anyhow::Result;

#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct PackageVersion(String);

impl From<String> for PackageVersion {
    fn from(s: String) -> Self {
        PackageVersion(s)
    }
}

/// A type which can be used to express a package version constraint
// TODO: Remove allow(unused)
#[derive(Debug, Eq, PartialEq)]
#[allow(unused)]
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
    // TODO: Remove allow(unused)
    #[allow(unused)]
    pub fn is_true(&self) -> bool {
        *self == PackageVersionMatch::True
    }

    pub fn is_false(&self) -> bool {
        *self == PackageVersionMatch::False
    }

    // TODO: Remove allow(unused)
    #[allow(unused)]
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

