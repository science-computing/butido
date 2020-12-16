use serde::Serialize;
use serde::Deserialize;

#[derive(parse_display::Display, Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
#[display("{0}")]
pub struct EnvironmentVariableName(String);

impl From<&str> for EnvironmentVariableName {
    fn from(s: &str) -> EnvironmentVariableName {
        EnvironmentVariableName(s.to_string())
    }
}

impl AsRef<str> for EnvironmentVariableName {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

pub mod docker;
pub mod filters;
pub mod git;
pub mod parser;
pub mod progress;

pub fn stdout_is_pipe() -> bool {
    !atty::is(atty::Stream::Stdout)
}

