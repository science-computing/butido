use anyhow::Result;
use anyhow::anyhow;
use lazy_static::lazy_static;
use regex::Regex;

use crate::package::PackageName;
use crate::package::PackageVersionConstraint;

mod build;
pub use build::*;

mod runtime;
pub use runtime::*;

mod system;
pub use system::*;

mod system_runtime;
pub use system_runtime::*;

pub trait StringEqual {
    fn str_equal(&self, s: &str) -> bool;
}

pub trait ParseDependency {
    fn parse_as_name_and_version(&self) -> Result<(PackageName, PackageVersionConstraint)>;
}

lazy_static! {
    pub(in crate::package::dependency)  static ref DEPENDENCY_PARSING_RE: Regex =
        Regex::new("^(?P<name>[[:alpha:]]([[[:alnum:]]-_])*) (?P<version>([\\*=><])?[[:alnum:]]([[[:alnum:]][[:punct:]]])*)$").unwrap();
}

/// Helper function for the actual implementation of the ParseDependency trait.
///
/// TODO: Reimplement using pom crate
pub(in crate::package::dependency) fn parse_package_dependency_string_into_name_and_version(s: &str)
    -> Result<(PackageName, PackageVersionConstraint)>
{
    let caps = crate::package::dependency::DEPENDENCY_PARSING_RE
        .captures(s)
        .ok_or_else(|| anyhow!("Could not parse into package name and package version constraint: '{}'", s))?;

    let name = caps.name("name")
        .ok_or_else(|| anyhow!("Could not parse name: '{}'", s))?;

    let vers = caps.name("version")
        .ok_or_else(|| anyhow!("Could not parse version: '{}'", s))?;

    let v = PackageVersionConstraint::parser().parse(vers.as_str().as_bytes())?;

    Ok((PackageName::from(String::from(name.as_str())), v))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::package::PackageName;
    use crate::package::PackageVersion;

    //
    // helper functions
    //

    fn name(s: &'static str) -> PackageName {
        PackageName::from(String::from(s))
    }

    fn exact(s: &'static str) -> PackageVersion {
        PackageVersion::from(String::from(s))
    }

    //
    // tests
    //

    #[test]
    fn test_dependency_conversion_1() {
        let s = "vim =8.2";
        let d = Dependency::from(String::from(s));

        let (n, c) = d.parse_as_name_and_version().unwrap();

        assert_eq!(n, name("vim"));
        assert_eq!(c, PackageVersionConstraint::from_version(String::from("="), exact("8.2")));
    }

    #[test]
    fn test_dependency_conversion_2() {
        let s = "gtk15 =1b";
        let d = Dependency::from(String::from(s));

        let (n, c) = d.parse_as_name_and_version().unwrap();

        assert_eq!(n, name("gtk15"));
        assert_eq!(c, PackageVersionConstraint::from_version(String::from("="), exact("1b")));
    }
}