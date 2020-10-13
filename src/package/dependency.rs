use std::result::Result as RResult;
use serde::Deserialize;
use regex::Regex;
use lazy_static::lazy_static;
use anyhow::anyhow;

use crate::package::PackageName;
use crate::package::PackageVersionConstraint;


#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct SystemDependency(String);

#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct BuildDependency(String);

#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct Dependency(String);

impl From<String> for Dependency {
    fn from(s: String) -> Dependency {
        Dependency(s)
    }
}

impl std::convert::TryInto<(PackageName, PackageVersionConstraint)> for Dependency {
    type Error = anyhow::Error;

    fn try_into(self) -> RResult<(PackageName, PackageVersionConstraint), Self::Error> {
        lazy_static! {
            static ref RE: Regex = Regex::new("^(?P<name>[[:alpha:]]([[[:alnum:]]-_])*) (?P<version>([\\*=><])?[[:alnum:]]([[[:alnum:]][[:punct:]]])*)$").unwrap();
        }

        let caps = RE.captures(&self.0)
            .ok_or_else(|| anyhow!("Could not parse into package name and package version constraint: '{}'", self.0))?;

        let name = caps.name("name")
            .ok_or_else(|| anyhow!("Could not parse name: '{}'", self.0))?;

        let vers = caps.name("version")
            .ok_or_else(|| anyhow!("Could not parse version: '{}'", self.0))?;

        let constraint = PackageVersionConstraint::parse(vers.as_str())?;

        Ok((PackageName::from(String::from(name.as_str())), constraint))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryInto;
    use crate::package::PackageVersion;
    use crate::package::PackageVersionConstraint;

    //
    // helper functions
    //

    fn name(s: &'static str) -> PackageName {
        PackageName::from(String::from(s))
    }

    fn exact(s: &'static str) -> PackageVersionConstraint {
        PackageVersionConstraint::Exact(PackageVersion::from(String::from(s)))
    }

    fn higher_as(s: &'static str) -> PackageVersionConstraint {
        PackageVersionConstraint::HigherAs(PackageVersion::from(String::from(s)))
    }

    //
    // tests
    //

    #[test]
    fn test_dependency_conversion_1() {
        let s = "vim =8.2";
        let d = Dependency::from(String::from(s));

        let (n, c) = d.try_into().unwrap();

        assert_eq!(n, name("vim"));
        assert_eq!(c, exact("8.2"));
    }

    #[test]
    fn test_dependency_conversion_2() {
        let s = "gtk15 >1b";
        let d = Dependency::from(String::from(s));

        let (n, c) = d.try_into().unwrap();

        assert_eq!(n, name("gtk15"));
        assert_eq!(c, higher_as("1b"));
    }
}
