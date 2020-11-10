use std::collections::BTreeMap;
use std::ops::Deref;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use handlebars::Handlebars;

use crate::config::NotValidatedConfiguration;

#[derive(Debug)]
pub struct Configuration<'reg> {
    pub (in crate::config) inner: NotValidatedConfiguration,
    pub (in crate::config) hb: Handlebars<'reg>,
}

impl<'reg> Deref for Configuration<'reg> {
    type Target = NotValidatedConfiguration;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'reg> Configuration<'reg> {
    /// Get the path to the releases directory, interpolate every variable used in the config
    pub fn releases_directory(&self, hm: &BTreeMap<String, String>) -> Result<PathBuf> {
        self.hb.render("releases", hm)
            .map(PathBuf::from)
            .context("Interpolating variables into 'release' setting from configuration")
    }

    /// Get the path to the staging directory, interpolate every variable used in the config
    pub fn staging_directory(&self, hm: &BTreeMap<String, String>) -> Result<PathBuf> {
        self.hb.render("staging", hm)
            .map(PathBuf::from)
            .context("Interpolating variables into 'staging' setting from configuration")
    }
}

