use serde::Deserialize;
use anyhow::anyhow;
use anyhow::Result;
use pom::parser::Parser as PomParser;

#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
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
        use crate::util::parser::*;
        (
            numbers() +
            ((dash() | under() | dot() | letters() | numbers()).repeat(0..))
        )
        .collect()
        .convert(|b| String::from_utf8(b.to_vec()).map(Self::from))
    }
}

/// A type which can be used to express a package version constraint
// TODO: Remove allow(unused)
#[derive(Debug, Eq, PartialEq)]
#[allow(unused)]
pub enum PackageVersionConstraint {
    Any,
    LowerAs(PackageVersion),
    HigherAs(PackageVersion),
    InRange(PackageVersion, PackageVersion),
    Exact(PackageVersion),
}

impl PackageVersionConstraint {
    pub fn matches(&self, v: &PackageVersion) -> Result<PackageVersionMatch> {
        match self {
            PackageVersionConstraint::Any                     => Ok(PackageVersionMatch::True),
            PackageVersionConstraint::LowerAs(_vers)          => Ok(PackageVersionMatch::Undecided), // TODO: Fix implementation
            PackageVersionConstraint::HigherAs(_vers)         => Ok(PackageVersionMatch::Undecided), // TODO: Fix implementation
            PackageVersionConstraint::InRange(_vers1, _vers2) => Ok(PackageVersionMatch::Undecided), // TODO: Fix implementation
            PackageVersionConstraint::Exact(vers)             => Ok(PackageVersionMatch::from(*v == *vers)),
        }
    }

    // TODO: Make this nice?
    pub fn parse(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Err(anyhow!("Cannot parse: '{}'", s))
        }

        let first_char = s.chars().next().ok_or_else(|| anyhow!("Failed to find first character: '{}'", s))?;
        if first_char == '*' {
            return Ok(PackageVersionConstraint::Any)
        }

        let v = s.chars().skip(1).collect::<String>();

        if v.is_empty() {
            return Err(anyhow!("Not a version: '{}'", v))
        }

        if first_char == '=' {
            Ok(PackageVersionConstraint::Exact(PackageVersion::from(v)))
        } else if first_char == '>' {
            Ok(PackageVersionConstraint::HigherAs(PackageVersion::from(v)))
        } else if first_char == '<' {
            Ok(PackageVersionConstraint::LowerAs(PackageVersion::from(v)))
        } else {
            let mut iter = s.split("..");

            let a = iter.next()
                .map(String::from)
                .ok_or_else(|| anyhow!("Trying to parse version range constraint failed: '{}'", s))?;

            let b = iter.next()
                .map(String::from)
                .ok_or_else(|| anyhow!("Trying to parse version range constraint failed: '{}'", s))?;

            Ok(PackageVersionConstraint::InRange(PackageVersion::from(a), PackageVersion::from(b)))
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_constraint_1() {
        let s = "*";
        let c = PackageVersionConstraint::parse(s).unwrap();
        assert_eq!(c, PackageVersionConstraint::Any);
    }

    #[test]
    fn test_parse_version_constraint_2() {
        let s = "=1";
        let c = PackageVersionConstraint::parse(s).unwrap();
        assert_eq!(c, PackageVersionConstraint::Exact(PackageVersion::from(String::from("1"))));
    }

    #[test]
    fn test_parse_version_constraint_3() {
        let s = ">1";
        let c = PackageVersionConstraint::parse(s).unwrap();
        assert_eq!(c, PackageVersionConstraint::HigherAs(PackageVersion::from(String::from("1"))));
    }

    #[test]
    fn test_parse_version_constraint_4() {
        let s = "<1";
        let c = PackageVersionConstraint::parse(s).unwrap();
        assert_eq!(c, PackageVersionConstraint::LowerAs(PackageVersion::from(String::from("1"))));
    }

    #[test]
    fn test_parse_version_constraint_5() {
        let s = "=1.0.17";
        let c = PackageVersionConstraint::parse(s).unwrap();
        assert_eq!(c, PackageVersionConstraint::Exact(PackageVersion::from(String::from("1.0.17"))));
    }

    #[test]
    fn test_parse_version_constraint_6() {
        let s = "=1.0.17asejg";
        let c = PackageVersionConstraint::parse(s).unwrap();
        assert_eq!(c, PackageVersionConstraint::Exact(PackageVersion::from(String::from("1.0.17asejg"))));
    }

    #[test]
    fn test_parse_version_constraint_7() {
        let s = "=1-0B17-beta1247_commit_12653hasd";
        let c = PackageVersionConstraint::parse(s).unwrap();
        assert_eq!(c, PackageVersionConstraint::Exact(PackageVersion::from(String::from("1-0B17-beta1247_commit_12653hasd"))));
    }
}

