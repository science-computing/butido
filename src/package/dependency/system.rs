use serde::Deserialize;

use crate::package::dependency::StringEqual;

/// A dependency that can be installed from the system and is only required during build
#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct SystemBuildDependency(String);

impl StringEqual for SystemBuildDependency {
    fn str_equal(&self, s: &str) -> bool {
        self.0 == s
    }
}

