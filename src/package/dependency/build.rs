use serde::Serialize;
use serde::Deserialize;
use anyhow::Result;

use crate::package::dependency::StringEqual;
use crate::package::dependency::ParseDependency;
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
    fn parse_into_name_and_version(self) -> Result<(PackageName, PackageVersionConstraint)> {
        crate::package::dependency::parse_package_dependency_string_into_name_and_version(&self.0)
    }
}

