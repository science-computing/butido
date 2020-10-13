use std::ops::Deref;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct PackageName(String);

impl Deref for PackageName {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for PackageName {
    fn from(s: String) -> Self {
        PackageName(s)
    }
}

impl std::fmt::Display for PackageName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        self.0.fmt(f)
    }
}
