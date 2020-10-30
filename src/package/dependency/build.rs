use serde::Deserialize;

use crate::package::dependency::StringEqual;

/// A dependency that is packaged and is only required during build time
#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct BuildDependency(String);

impl StringEqual for BuildDependency {
    fn str_equal(&self, s: &str) -> bool {
        self.0 == s
    }
}
