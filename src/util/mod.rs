use serde::Deserialize;

#[derive(Deserialize, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct EnvironmentVariableName(String);

pub mod docker;
pub mod filters;
pub mod git;
pub mod parser;
pub mod progress;

pub fn stdout_is_pipe() -> bool {
    !atty::is(atty::Stream::Stdout)
}

