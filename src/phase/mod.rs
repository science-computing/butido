use std::path::PathBuf;

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Hash)]
#[serde(transparent)]
pub struct PhaseName(String);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum Phase {
    #[serde(rename = "path")]
    Path(PathBuf),

    #[serde(rename = "script")]
    Text(String),
}

