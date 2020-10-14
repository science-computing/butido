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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum Phase {
    #[serde(rename = "path")]
    Path(PathBuf),

    #[serde(rename = "script")]
    Text(String),
}

