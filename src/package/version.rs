//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::ops::Deref;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use pom::parser::Parser as PomParser;
use serde::Deserialize;
use serde::Serialize;

use crate::util::parser::*;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct PackageVersionConstraint {
    constraint: String,
    version: PackageVersion,
}

impl PackageVersionConstraint {
    fn parser<'a>() -> PomParser<'a, u8, Self> {
        (pom::parser::sym(b'=').opt() + PackageVersion::parser())
            .convert(|(constraint, version)| {
                if let Some(c) = constraint {
                    String::from_utf8(vec![c]).map(|c| (c, version))
                } else {
                    Ok(("".to_string(), version))
                }
            })
            .map(|(constraint, version)| PackageVersionConstraint {
                constraint,
                version,
            })
    }

    pub fn matches(&self, v: &PackageVersion) -> bool {
        use semver::{Version, VersionReq};
        match self.constraint.as_str() {
            "" => {
                // We default to `Op::Exact` (=) to enable partial version specification
                // (e.g., only the major and minor or only the major version):
                // https://docs.rs/semver/latest/semver/enum.Op.html#opexact
                // Our own implementation of "=" (s. below) allows only a single version to match
                // while `semver::Op::Exact` can result in multiple versions matching if only a
                // "partial" version is provided.
                let default_constraint = String::from("=");
                let constraint =
                    VersionReq::parse(&(default_constraint + self.version.as_str())).unwrap();
                let version = Version::parse(v.as_str()).unwrap();

                constraint.matches(&version)
            }
            "=" => self.version == *v,
            _ => panic!(
                "Internal error: Unsupported version constraint: {} (version: {})",
                self.constraint, self.version
            ),
        }
    }

    #[cfg(test)]
    pub fn from_version(constraint: String, version: PackageVersion) -> Self {
        PackageVersionConstraint {
            constraint,
            version,
        }
    }
}

impl TryFrom<String> for PackageVersionConstraint {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self> {
        Self::try_from(&s as &str)
    }
}

impl TryFrom<&str> for PackageVersionConstraint {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        PackageVersionConstraint::parser()
            .parse(s.as_bytes())
            .context(anyhow!("Failed to parse the following package version constraint: {}", s))
            .context("A package version constraint must have a version and an optional comparator (only `=` is currently supported, which is also the default), e.g.: =0.1.0")
            .map_err(Error::from)
    }
}

impl std::fmt::Display for PackageVersionConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.constraint, self.version)
    }
}

#[derive(
    parse_display::Display,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Hash,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
)]
#[serde(transparent)]
#[display("{0}")]
pub struct PackageVersion(String);

impl Deref for PackageVersion {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for PackageVersion {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for PackageVersion {
    fn from(s: String) -> Self {
        PackageVersion(s)
    }
}

impl PackageVersion {
    fn parser<'a>() -> PomParser<'a, u8, Self> {
        (numbers() + ((dash() | under() | dot() | letters() | numbers()).repeat(0..)))
            .collect()
            .convert(|b| String::from_utf8(b.to_vec()).map(Self::from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_1() {
        assert!(PackageVersion::parser().parse(b"1").is_ok());
        assert!(PackageVersion::parser().parse(b"1.42").is_ok());
        assert!(PackageVersion::parser().parse(b"1.42.37").is_ok());

        assert!(PackageVersion::parser().parse(b"").is_err());
        assert!(PackageVersion::parser().parse(b"=").is_err());
        assert!(PackageVersion::parser().parse(b"*1").is_err());
        assert!(PackageVersion::parser().parse(b">1").is_err());
        assert!(PackageVersion::parser().parse(b"<1").is_err());
        assert!(PackageVersion::parser().parse(b"=a").is_err());
        assert!(PackageVersion::parser().parse(b"=.a").is_err());
        assert!(PackageVersion::parser().parse(b"=.1").is_err());
        assert!(PackageVersion::parser().parse(b"=a1").is_err());
        assert!(PackageVersion::parser().parse(b"a").is_err());

        assert!(PackageVersionConstraint::parser().parse(b"1").is_ok());
        assert!(PackageVersionConstraint::parser().parse(b"1.42").is_ok());
        assert!(PackageVersionConstraint::parser().parse(b"1.42.37").is_ok());

        assert!(PackageVersionConstraint::parser().parse(b"").is_err());
        assert!(PackageVersionConstraint::parser().parse(b"=").is_err());
        assert!(PackageVersionConstraint::parser().parse(b"*1").is_err());
        assert!(PackageVersionConstraint::parser().parse(b">1").is_err());
        assert!(PackageVersionConstraint::parser().parse(b"<1").is_err());
        assert!(PackageVersionConstraint::parser().parse(b"=a").is_err());
        assert!(PackageVersionConstraint::parser().parse(b"=.a").is_err());
        assert!(PackageVersionConstraint::parser().parse(b"=.1").is_err());
        assert!(PackageVersionConstraint::parser().parse(b"=a1").is_err());
        assert!(PackageVersionConstraint::parser().parse(b"a").is_err());
    }

    #[test]
    fn test_parse_version_2() {
        let s = "=1";
        let c = PackageVersionConstraint::parser()
            .parse(s.as_bytes())
            .unwrap();
        assert_eq!(c.version, PackageVersion::from(String::from("1")));
    }

    #[test]
    fn test_parse_version_3() {
        let s = "=1.0.17";
        let c = PackageVersionConstraint::parser()
            .parse(s.as_bytes())
            .unwrap();
        assert_eq!(c.version, PackageVersion::from(String::from("1.0.17")));
    }

    #[test]
    fn test_parse_version_4() {
        let s = "=1.0.17asejg";
        let c = PackageVersionConstraint::parser()
            .parse(s.as_bytes())
            .unwrap();
        assert_eq!(c.version, PackageVersion::from(String::from("1.0.17asejg")));
    }

    #[test]
    fn test_parse_version_5() {
        let s = "=1-0B17-beta1247_commit_12653hasd";
        let c = PackageVersionConstraint::parser()
            .parse(s.as_bytes())
            .unwrap();
        assert_eq!(
            c.version,
            PackageVersion::from(String::from("1-0B17-beta1247_commit_12653hasd"))
        );
    }
}
