use thiserror::Error as ThisError;

use crate::util::docker::ContainerHash;

#[derive(ThisError, Debug)]
pub enum ContainerError {
    #[error("Error during container run, connect using `docker --host {uri} exec -it {container_id} /bin/bash`")]
    ContainerError {
        container_id: ContainerHash,
        uri: String,
    },
}

impl ContainerError {
    pub fn container_error(container_id: ContainerHash, uri: String) -> Self {
        ContainerError::ContainerError { container_id, uri }
    }
}

