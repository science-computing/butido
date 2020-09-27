use std::path::PathBuf;
use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq, Hash)]
pub struct PhaseName(String);

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub enum Phase {
    Path(PathBuf),
    Text(String),
}

