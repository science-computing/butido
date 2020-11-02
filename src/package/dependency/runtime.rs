use serde::Deserialize;
use anyhow::Result;

use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::package::dependency::StringEqual;
use crate::package::dependency::ParseDependency;


/// A dependency that is packaged and is required during runtime
#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
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
    fn parse_into_name_and_version(self) -> Result<(PackageName, PackageVersionConstraint)> {
        crate::package::dependency::parse_package_dependency_string_into_name_and_version(&self.0)
    }
}

