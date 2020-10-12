use std::path::Path;
use anyhow::Result;

use crate::package::Dependency;

pub trait Executor {
    fn execute_dependency_script(&self, scriptpath: &Path) -> Result<Vec<Dependency>>;
}

