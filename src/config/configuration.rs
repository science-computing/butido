use std::collections::BTreeMap;
use std::ops::Deref;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use handlebars::Handlebars;

use crate::config::NotValidatedConfiguration;

#[derive(Debug)]
pub struct Configuration {
    pub (in crate::config) inner: NotValidatedConfiguration,
}

impl Deref for Configuration {
    type Target = NotValidatedConfiguration;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

