use std::result::Result as RResult;
use serde::Deserialize;
use regex::Regex;
use lazy_static::lazy_static;
use anyhow::anyhow;

use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::package::dependency::StringEqual;


/// A dependency that is packaged and is required during runtime
#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct Dependency(String);

impl StringEqual for Dependency {
    fn str_equal(&self, s: &str) -> bool {
        self.0 == s
    }
}

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

