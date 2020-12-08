use getset::CopyGetters;
use getset::Getters;
use serde::Deserialize;

use crate::util::EnvironmentVariableName;

#[derive(Debug, CopyGetters, Getters, Deserialize)]
pub struct ContainerConfig {
    #[getset(get_copy = "pub")]
    check_env_names: bool,

    #[getset(get = "pub")]
    allowed_env: Vec<EnvironmentVariableName>,
}

