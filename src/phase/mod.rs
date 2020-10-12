use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq, Hash)]
#[serde(transparent)]
pub struct PhaseName(String);

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub enum Phase {
    #[serde(rename = "path")]
    Path(PathBuf),

    #[serde(rename = "script")]
    Text(String),
}

