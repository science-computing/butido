//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::anyhow;
use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;

use crate::package::PackageName;
use crate::package::PackageVersionConstraint;

mod build;
pub use build::*;

mod runtime;
pub use runtime::*;

pub mod condition;

pub trait ParseDependency {
    fn parse_as_name_and_version(&self) -> Result<(PackageName, PackageVersionConstraint)>;
}

lazy_static! {
    // The following regex could be simplified significantly since we basically only need the space
    // (" ") for splitting name and version (and both shouldn't be empty) - the rest of the
    // validation could and probably should be done when parsing `name` and `version` (can make the
    // errors more precise and we avoid that the regex diverges from the rest of the validation as
    // it's already the case):
    pub(in crate::package::dependency)  static ref DEPENDENCY_PARSING_RE: Regex =
        Regex::new("^(?P<name>[[:alnum:]][[:alnum:]._-]*) (?P<version>[*=><]?[[:alnum:]][[:alnum:][:punct:]]*)$").unwrap();
}

/// Helper function for the actual implementation of the ParseDependency trait.
///
/// TODO: Reimplement using pom crate
pub(in crate::package::dependency) fn parse_package_dependency_string_into_name_and_version(
    s: &str,
) -> Result<(PackageName, PackageVersionConstraint)> {
    let caps = crate::package::dependency::DEPENDENCY_PARSING_RE
        .captures(s)
        .ok_or_else(|| {
            anyhow!(
                "Could not parse into package name and package version constraint: '{}'",
                s
            )
        })?;

    let name = caps
        .name("name")
        .map(|m| String::from(m.as_str()))
        .ok_or_else(|| anyhow!("Could not parse name: '{}'", s))?;

    let vers = caps
        .name("version")
        .map(|m| String::from(m.as_str()))
        .ok_or_else(|| anyhow!("Could not parse version: '{}'", s))?;

    let v = PackageVersionConstraint::try_from(vers).map_err(|e| {
        e.context(anyhow!(
            "Could not parse the following package dependency string: {}",
            s
        ))
    })?;
    Ok((PackageName::from(name), v))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::package::PackageVersion;

    //
    // helper functions
    //

    fn dep_parse_test(name: &'static str, version: &'static str) {
        let name = name.to_string();
        let version = version.to_string();

        let dependency_specification = format!("{name} ={version}");
        let dep = Dependency::from(dependency_specification.clone());
        let (dep_name, dep_version_constraint) = dep.parse_as_name_and_version().unwrap();

        let version_constraint = PackageVersionConstraint::from_version(
            String::from("="),
            PackageVersion::from(version),
        );
        assert_eq!(
            dep_name,
            PackageName::from(name),
            "Name check failed for input: {dependency_specification}"
        );
        assert_eq!(
            dep_version_constraint, version_constraint,
            "Version constraint check failed for input: {dependency_specification}"
        );
    }

    fn dep_parse_expect_err(dependency_specification: &'static str) {
        let dep = Dependency::from(dependency_specification.to_string());
        let result = dep.parse_as_name_and_version();
        assert!(
            result.is_err(),
            "Should not be able to parse this input: {dependency_specification}"
        );
    }

    //
    // tests
    //

    #[test]
    fn test_dependency_conversion_1() {
        dep_parse_test("vim", "8.2");
    }

    #[test]
    fn test_dependency_conversion_2() {
        dep_parse_test("gtk15", "1b");
    }

    #[test]
    fn test_dependency_string_with_punctuation() {
        dep_parse_test("foo-bar1.2.3", "0.123");
    }

    #[test]
    fn test_dependency_string_where_pkg_starts_with_number() {
        dep_parse_test("7z", "42");
    }

    #[test]
    fn test_dependency_version_without_constraint() {
        let name = "foobar";
        let version_constraint = "1.42.37";

        let dep = Dependency::from(format!("{name} {version_constraint}"));
        let (dep_name, dep_version_constraint) = dep.parse_as_name_and_version().unwrap();

        assert_eq!(dep_name, PackageName::from(name.to_string()));
        assert_eq!(
            dep_version_constraint,
            PackageVersionConstraint::from_version(
                String::from("="),
                PackageVersion::from(version_constraint.to_string()),
            )
        );
    }

    #[test]
    fn test_complex_dependency_parsing() {
        dep_parse_test("0ad_", "42");
        dep_parse_test("2048-cli_0.0", "42");

        dep_parse_expect_err("0] =42");
        dep_parse_expect_err("a\\ =42");
        dep_parse_expect_err("a =.0");
        dep_parse_expect_err("a =");
        dep_parse_expect_err("");
        dep_parse_expect_err(" ");
        // Not supported yet:
        dep_parse_expect_err("a *");
        dep_parse_expect_err("a >2");
        dep_parse_expect_err("a <2");
    }
}
