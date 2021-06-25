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

use crate::package::dependency::ParseDependency;
use crate::package::dependency::StringEqual;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;

/// A dependency that is packaged and is only required during build time
#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(untagged)]
pub enum BuildDependency {
    Simple(String),
}

impl AsRef<str> for BuildDependency {
    fn as_ref(&self) -> &str {
        match self {
            BuildDependency::Simple(name) => name,
        }
    }
}

impl StringEqual for BuildDependency {
    fn str_equal(&self, s: &str) -> bool {
        match self {
            BuildDependency::Simple(name) => name == s,
        }
    }
}

impl ParseDependency for BuildDependency {
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
        setting: BuildDependency,
    }

    #[test]
    fn test_parse_dependency() {
        let s: TestSetting = toml::from_str(r#"setting = "foo""#).expect("Parsing TestSetting failed");
        match s.setting {
            BuildDependency::Simple(name) => assert_eq!(name, "foo", "Expected 'foo', got {}", name),
        }
    }
}

