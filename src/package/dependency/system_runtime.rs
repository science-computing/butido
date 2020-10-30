use serde::Deserialize;

use crate::package::dependency::StringEqual;

/// A dependency that can be installed from the system and is required during runtime
#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct SystemDependency(String);

impl StringEqual for SystemDependency {
    fn str_equal(&self, s: &str) -> bool {
        self.0 == s
    }
}

