//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::ops::Deref;

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
        (pom::parser::sym(b'=') + PackageVersion::parser())
            .convert(|(constraint, version)| {
                String::from_utf8(vec![constraint]).map(|c| (c, version))
            })
            .map(|(constraint, version)| PackageVersionConstraint {
                constraint,
                version,
            })
    }

    pub fn matches(&self, v: &PackageVersion) -> bool {
        self.version == *v
    }

    #[cfg(test)]
    pub fn from_version(constraint: String, version: PackageVersion) -> Self {
        PackageVersionConstraint {
            constraint,
            version,
        }
    }
}

impl std::convert::TryFrom<String> for PackageVersionConstraint {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self> {
        Self::try_from(&s as &str)
    }
}

impl std::convert::TryFrom<&str> for PackageVersionConstraint {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        PackageVersionConstraint::parser()
            .parse(s.as_bytes())
            .map_err(Error::from)
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
    pub fn parser<'a>() -> PomParser<'a, u8, Self> {
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

        assert!(PackageVersionConstraint::parser()
            .parse(b"")
            .is_err());
        assert!(PackageVersionConstraint::parser()
            .parse(b"=")
            .is_err());
        assert!(PackageVersionConstraint::parser()
            .parse(b"*1")
            .is_err());
        assert!(PackageVersionConstraint::parser()
            .parse(b">1")
            .is_err());
        assert!(PackageVersionConstraint::parser()
            .parse(b"<1")
            .is_err());
        assert!(PackageVersionConstraint::parser()
            .parse(b"=a")
            .is_err());
        assert!(PackageVersionConstraint::parser()
            .parse(b"=.a")
            .is_err());
        assert!(PackageVersionConstraint::parser()
            .parse(b"=.1")
            .is_err());
        assert!(PackageVersionConstraint::parser()
            .parse(b"=a1")
            .is_err());
        assert!(PackageVersionConstraint::parser()
            .parse(b"1")
            .is_err());
        assert!(PackageVersionConstraint::parser()
            .parse(b"a")
            .is_err());
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
