use std::path::PathBuf;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Hash)]
#[serde(transparent)]
pub struct PhaseName(String);

impl PhaseName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
impl From<String> for PhaseName {
    fn from(s: String) -> Self {
        PhaseName(s)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum Phase {
    #[serde(rename = "path")]
    Path(PathBuf),

    #[serde(rename = "script")]
    Text(String),
}

