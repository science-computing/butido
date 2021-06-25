//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::package::dependency::ParseDependency;
use crate::package::dependency::StringEqual;
use crate::package::dependency::condition::Condition;

/// A dependency that is packaged and is required during runtime
#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(untagged)]
pub enum Dependency {
    Simple(String),
    Conditional(String, Condition),
}

impl AsRef<str> for Dependency {
    fn as_ref(&self) -> &str {
        match self {
            Dependency::Simple(name) => name,
            Dependency::Conditional(name, _) => name,
        }
    }
}

impl StringEqual for Dependency {
    fn str_equal(&self, s: &str) -> bool {
        match self {
            Dependency::Simple(name) => name == s,
            Dependency::Conditional(name, _) => name == s,
        }
    }
}

impl From<String> for Dependency {
    fn from(s: String) -> Dependency {
        Dependency::Simple(s)
    }
}

impl ParseDependency for Dependency {
    fn parse_as_name_and_version(&self) -> Result<(PackageName, PackageVersionConstraint)> {
        crate::package::dependency::parse_package_dependency_string_into_name_and_version(self.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    #[allow(unused)]
    pub struct TestSetting {
        setting: Dependency,
    }

    #[test]
    fn test_parse_dependency() {
        let s: TestSetting = toml::from_str(r#"setting = "foo""#).expect("Parsing TestSetting failed");

        match s.setting {
            Dependency::Simple(name) => assert_eq!(name, "foo", "Expected 'foo', got {}", name),
            other => panic!("Unexpected deserialization to other variant: {:?}", other),
        }
    }
}

