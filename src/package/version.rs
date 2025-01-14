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
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use tracing::info;

use crate::util::parser::*;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct PackageVersionConstraint {
    constraint: String,
    version: PackageVersion,
}

impl PackageVersionConstraint {
    fn get_default_constraint() -> String {
        // We default to `Op::Exact` (=) to enable partial version specification
        // (e.g., only the major and minor or only the major version):
        // https://docs.rs/semver/latest/semver/enum.Op.html#opexact
        // Our own implementation of "=" (s. below) allows only a single version to match
        // while `semver::Op::Exact` can result in multiple versions matching if only a
        // "partial" version is provided.
        String::from("=")
    }

    fn parser<'a>() -> PomParser<'a, u8, Self> {
        (pom::parser::sym(b'=').opt() + PackageVersion::parser())
            .convert(|(constraint, version)| {
                if let Some(c) = constraint {
                    String::from_utf8(vec![c])
                        .map(|c| (c, version))
                        .map_err(Error::from)
                } else {
                    semver::VersionReq::parse(&(Self::get_default_constraint() + &version))
                        .map(|_| ("".to_string(), version.clone()))
                        .map_err(Error::from)
                        // TODO: Drop this (for backward compatibility, we temporarily fallback to
                        // the old behaviour (as if the constraint `=` was specified) if the
                        // provided version cannot be parsed by the semver crate - this is required
                        // for somewhat "exotic" versions like the old OpenSSL 1.1.1w, web browsers
                        // with a fourth version number, or (unstable) releases based on the date):
                        .inspect_err(|e| info!("Couldn't parse version \"{version}\" as SemVer ({e}) -> falling back to strict version matching (={version})"))
                        .map_or(Ok(("=".to_string(), version)), Ok)
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
                let constraint =
                    VersionReq::parse(&(Self::get_default_constraint() + self.version.as_str()))
                        .unwrap();
                let version = Version::parse(v.as_str())
                    .with_context(|| anyhow!("Failed to parse the package version as semver::Version"))
                    .or_else(|eo| v.clone().try_into().map_err(|e: Error| e.context(eo)))
                    .with_context(|| anyhow!("Also failed to parse the package version using our own SemVer converter"))
                    .unwrap_or_else(|e| panic!(
                        "Failed to parse the package version \"{}\" as SemVer to check if it matches \"{}\". Error: {:#}",
                        v,
                        constraint,
                        e
                    ));

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
    #[allow(unused)]
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
        PackageVersion(s.trim_start_matches('=').to_string())
    }
}

impl TryInto<semver::Version> for PackageVersion {
    type Error = anyhow::Error;

    // TODO: Improve or replace this spaghetti code that we only use as a fallback (the unwrap()s
    // should be safe as long as the static regex guarantees the assumptions):
    fn try_into(self) -> Result<semver::Version> {
        // Warning: This regex must remain compatible to the one in the PackageVersion::parser below!:
        let version_regex = Regex::new("^(?:([[:digit:]]+)|[-_.]|[[:alpha:]]+)").unwrap();
        // This regex is based on PackageVersion::parser() below. We use a capture group to extract
        // the numbers and the "(?:exp)" syntax is for a non-capturing group. If it matches we'll
        // have the entire match in \0 and if it's a number it'll be in \1.

        // Our input (version string):
        let mut version_str = self.0.as_str();
        // Our results (extracted version numbers):
        let mut versions = Vec::<u64>::new();

        // This loop is dangerous... Ensure that the match gets removed from version_str in every
        // iteration to avoid an endless loop!
        while let Some(captures) = version_regex.captures(version_str) {
            // For debugging: println!("{:?}", captures);

            let match_str = captures.get(0).unwrap().as_str(); // Unwrap safe as \0 always exists
            version_str = &version_str[match_str.len()..];

            if let Some(version_match) = captures.get(1) {
                // We have a non-empty (version) number match
                let version = version_match.as_str().parse()?;
                versions.push(version);
            }
        }

        if version_str.is_empty() {
            // Return what is hopefully the corresponding SemVer (the minor and patch version
            // default to zero if they couldn't be extracted from PackageVersion):
            Ok(semver::Version::new(
                *versions.first().unwrap_or(&0),
                *versions.get(1).unwrap_or(&0),
                *versions.get(2).unwrap_or(&0),
            ))
        } else {
            // We couldn't parse the entire version string -> report an error:
            Err(anyhow!(
                "The following rest of the package version couldn't be parsed: {}",
                version_str
            ))
            .with_context(|| {
                anyhow!(
                    "The regex \"{}\" for parsing the PackageVersion didn't match",
                    version_regex
                )
            })
            .with_context(|| {
                anyhow!(
                    "The PackageVersion \"{}\" couldn't be converted into a semver::Version",
                    self
                )
            })
        }
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
