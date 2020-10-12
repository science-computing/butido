use std::path::Path;
use anyhow::Result;

use crate::package::Dependency;

pub trait Executor {
    fn execute_dependency_script(&self, scriptpath: &Path) -> Result<Vec<Dependency>>;
}

pub struct DummyExecutor;
impl DummyExecutor {
    pub fn new() -> Self {
        DummyExecutor
    }
}

impl Executor for DummyExecutor {
    fn execute_dependency_script(&self, _scriptpath: &Path) -> Result<Vec<Dependency>> {
        Ok(vec![])
    }
}

