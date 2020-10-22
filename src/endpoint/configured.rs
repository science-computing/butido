use std::fmt::{Debug, Formatter};

use shiplift::Docker;
use getset::{Getters, CopyGetters};
use typed_builder::TypedBuilder;

#[derive(Getters, CopyGetters, TypedBuilder, Clone)]
pub struct ConfiguredEndpoint {
    #[getset(get = "pub")]
    name: String,

    #[getset(get = "pub")]
    docker: Docker,

    #[getset(get_copy = "pub")]
    speed: usize,

    #[getset(get_copy = "pub")]
    num_current_jobs: usize,

    #[getset(get_copy = "pub")]
    num_max_jobs: usize,
}

impl Debug for ConfiguredEndpoint {
    fn fmt(&self, f: &mut Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f,
            "ConfiguredEndpoint({}, {}/{})",
            self.name,
            self.num_current_jobs,
            self.num_max_jobs)
    }
}

