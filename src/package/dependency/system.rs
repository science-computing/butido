use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::package::dependency::ParseDependency;
use crate::package::dependency::StringEqual;

/// A dependency that can be installed from the system and is only required during build
#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct SystemBuildDependency(String);

impl AsRef<str> for SystemBuildDependency {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl StringEqual for SystemBuildDependency {
    fn str_equal(&self, s: &str) -> bool {
        self.0 == s
    }
}

impl ParseDependency for SystemBuildDependency {
    fn parse_as_name_and_version(&self) -> Result<(PackageName, PackageVersionConstraint)> {
        crate::package::dependency::parse_package_dependency_string_into_name_and_version(&self.0)
    }
}

