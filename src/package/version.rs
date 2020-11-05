use serde::Serialize;
use serde::Deserialize;
use anyhow::anyhow;
use anyhow::Error;
use anyhow::Result;
use pom::parser::Parser as PomParser;
use crate::util::parser::*;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct PackageVersionConstraint {
    constraint: String,
    version: PackageVersion
}

impl PackageVersionConstraint {
    pub fn parser<'a>() -> PomParser<'a, u8, Self> {
        (pom::parser::sym(b'=') + PackageVersion::parser())
            .convert(|(constraint, version)| {
                String::from_utf8(vec![constraint])
                    .map(|c| (c, version))
            })
            .map(|(constraint, version)| PackageVersionConstraint { constraint, version })
    }

    pub fn matches(&self, v: &PackageVersion) -> bool {
        self.version == *v
    }

    #[cfg(test)]
    pub fn from_version(constraint: String, version: PackageVersion) -> Self {
        PackageVersionConstraint { constraint, version }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct PackageVersion(String);

impl From<String> for PackageVersion {
    fn from(s: String) -> Self {
        PackageVersion(s)
    }
}

impl std::fmt::Display for PackageVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        self.0.fmt(f)
    }
}

impl PackageVersion {
    pub fn parser<'a>() -> PomParser<'a, u8, Self> {
        (
            numbers() + ((dash() | under() | dot() | letters() | numbers()).repeat(0..))
        )
        .collect()
        .convert(|b| String::from_utf8(b.to_vec()).map(Self::from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_1() {
        assert!(PackageVersion::parser().parse("".as_bytes()).is_err());
        assert!(PackageVersion::parser().parse("=".as_bytes()).is_err());
        assert!(PackageVersion::parser().parse("*1".as_bytes()).is_err());
        assert!(PackageVersion::parser().parse(">1".as_bytes()).is_err());
        assert!(PackageVersion::parser().parse("<1".as_bytes()).is_err());
        assert!(PackageVersion::parser().parse("=a".as_bytes()).is_err());
        assert!(PackageVersion::parser().parse("=.a".as_bytes()).is_err());
        assert!(PackageVersion::parser().parse("=.1".as_bytes()).is_err());
        assert!(PackageVersion::parser().parse("=a1".as_bytes()).is_err());
        assert!(PackageVersion::parser().parse("a".as_bytes()).is_err());

        assert!(PackageVersionConstraint::parser().parse("".as_bytes()).is_err());
        assert!(PackageVersionConstraint::parser().parse("=".as_bytes()).is_err());
        assert!(PackageVersionConstraint::parser().parse("*1".as_bytes()).is_err());
        assert!(PackageVersionConstraint::parser().parse(">1".as_bytes()).is_err());
        assert!(PackageVersionConstraint::parser().parse("<1".as_bytes()).is_err());
        assert!(PackageVersionConstraint::parser().parse("=a".as_bytes()).is_err());
        assert!(PackageVersionConstraint::parser().parse("=.a".as_bytes()).is_err());
        assert!(PackageVersionConstraint::parser().parse("=.1".as_bytes()).is_err());
        assert!(PackageVersionConstraint::parser().parse("=a1".as_bytes()).is_err());
        assert!(PackageVersionConstraint::parser().parse("1".as_bytes()).is_err());
        assert!(PackageVersionConstraint::parser().parse("a".as_bytes()).is_err());
    }

    #[test]
    fn test_parse_version_2() {
        let s = "=1";
        let c = PackageVersionConstraint::parser().parse(s.as_bytes()).unwrap();
        assert_eq!(c.version, PackageVersion::from(String::from("1")));
    }

    #[test]
    fn test_parse_version_3() {
        let s = "=1.0.17";
        let c = PackageVersionConstraint::parser().parse(s.as_bytes()).unwrap();
        assert_eq!(c.version, PackageVersion::from(String::from("1.0.17")));
    }

    #[test]
    fn test_parse_version_4() {
        let s = "=1.0.17asejg";
        let c = PackageVersionConstraint::parser().parse(s.as_bytes()).unwrap();
        assert_eq!(c.version, PackageVersion::from(String::from("1.0.17asejg")));
    }

    #[test]
    fn test_parse_version_5() {
        let s = "=1-0B17-beta1247_commit_12653hasd";
        let c = PackageVersionConstraint::parser().parse(s.as_bytes()).unwrap();
        assert_eq!(c.version, PackageVersion::from(String::from("1-0B17-beta1247_commit_12653hasd")));
    }
}

