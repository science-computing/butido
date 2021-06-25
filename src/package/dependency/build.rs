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
#[serde(transparent)]
pub struct BuildDependency(String);

impl AsRef<str> for BuildDependency {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl StringEqual for BuildDependency {
    fn str_equal(&self, s: &str) -> bool {
        self.0 == s
    }
}

impl ParseDependency for BuildDependency {
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
        setting: BuildDependency,
    }

    #[test]
    fn test_parse_dependency() {
        let dependency_str = r#"setting = "foo""#;
        let d: TestSetting = toml::from_str(dependency_str).unwrap();

        assert_eq!(d.setting.0, "foo");
    }
}

