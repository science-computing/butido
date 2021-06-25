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

/// A dependency that is packaged and is required during runtime
#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct Dependency(String);

impl AsRef<str> for Dependency {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

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

impl ParseDependency for Dependency {
    fn parse_as_name_and_version(&self) -> Result<(PackageName, PackageVersionConstraint)> {
        crate::package::dependency::parse_package_dependency_string_into_name_and_version(&self.0)
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

        assert_eq!(s.setting.0, "foo");
    }
}

