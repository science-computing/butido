use getset::Getters;
use serde::Deserialize;

use crate::util::EnvironmentVariableName;

#[derive(Debug, Getters, Deserialize)]
pub struct ContainerConfig {
    #[getset(get = "pub")]
    allowed_env: Vec<EnvironmentVariableName>,
}

