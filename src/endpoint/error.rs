use thiserror::Error as ThisError;

use crate::util::docker::ContainerHash;
use crate::package::Script;

#[derive(ThisError, Debug)]
pub enum ContainerError {

    #[error("Error during container run: {container_id}")]
    ContainerError {
        container_id: ContainerHash,
    },

    #[error("{0}")]
    Err(anyhow::Error),
}

impl ContainerError {
    pub fn container_error(container_id: ContainerHash) -> Self {
        ContainerError::ContainerError { container_id }
    }

    pub fn explain_container_error(&self) -> Option<String> {
        match self {
            ContainerError::ContainerError { container_id } => Some({
                indoc::formatdoc!(r#"
                    Container did not exit successfully: {container_id}
                    Use

                        docker exec -it {container_id} /bin/bash

                    to access and debug.
                    "#, container_id = container_id)
            }),
            _ => None,
        }
    }
}

impl From<anyhow::Error> for ContainerError {
    fn from(ae: anyhow::Error) -> Self {
        ContainerError::Err(ae)
    }
}

