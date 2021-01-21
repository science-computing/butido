//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::collections::HashMap;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use getset::Getters;
use log::debug;
use uuid::Uuid;

use crate::config::Configuration;
use crate::filestore::Artifact;
use crate::job::Job;
use crate::job::JobResource;
use crate::package::Package;
use crate::package::Script;
use crate::package::ScriptBuilder;
use crate::source::SourceCache;
use crate::source::SourceEntry;
use crate::util::EnvironmentVariableName;
use crate::util::docker::ImageName;

/// A job configuration that can be run. All inputs are clear here.
#[derive(Debug, Getters)]
pub struct RunnableJob {
    #[getset(get = "pub")]
    uuid: Uuid,

    #[getset(get = "pub")]
    package: Package,

    #[getset(get = "pub")]
    image: ImageName,

    #[getset(get = "pub")]
    source_cache: SourceCache,

    #[getset(get = "pub")]
    script: Script,

    #[getset(get = "pub")]
    resources: Vec<JobResource>,
}

impl RunnableJob {
    pub async fn build_from_job(
        job: &Job,
        source_cache: &SourceCache,
        config: &Configuration,
        dependencies: Vec<Artifact>,
    ) -> Result<Self> {
        // Add the environment from the original Job object to the resources
        let resources = dependencies
            .into_iter()
            .map(JobResource::from)
            .chain({
                job.resources()
                    .iter()
                    .filter(|jr| jr.env().is_some())
                    .cloned()
            })
            .collect();

        if config.containers().check_env_names() {
            debug!("Checking environment if all variables are allowed!");
            let _ = Self::env_resources(job.resources(), job.package().environment().as_ref())
                .into_iter()
                .inspect(|(name, _)| debug!("Checking: {}", name))
                .try_for_each(|(name, _)| {
                    if !config.containers().allowed_env().contains(&name) {
                        Err(anyhow!("Environment variable name not allowed: {}", name))
                    } else {
                        Ok(())
                    }
                })
                .with_context(|| {
                    anyhow!(
                        "Checking allowed variables for package {} {}",
                        job.package().name(),
                        job.package().version()
                    )
                })
                .context("Checking allowed variable names")?;
        } else {
            debug!("Environment checking disabled");
        }

        let script = ScriptBuilder::new(&job.script_shebang).build(
            &job.package,
            &job.script_phases,
            *config.strict_script_interpolation(),
        )?;

        Ok(RunnableJob {
            uuid: job.uuid,
            package: job.package.clone(),
            image: job.image.clone(),
            resources,
            source_cache: source_cache.clone(),

            script,
        })
    }

    pub fn package_sources(&self) -> Vec<SourceEntry> {
        self.source_cache.sources_for(&self.package())
    }

    pub fn environment(&self) -> Vec<(EnvironmentVariableName, String)> {
        Self::env_resources(&self.resources, self.package().environment().as_ref())
    }

    /// Helper function to collect a list of resources and the result of package.environment() into
    /// a Vec of environment variables
    fn env_resources(
        resources: &[JobResource],
        pkgenv: Option<&HashMap<EnvironmentVariableName, String>>,
    ) -> Vec<(EnvironmentVariableName, String)> {
        let iter = resources
            .iter()
            .filter_map(JobResource::env)
            .map(|(k, v)| (k.clone(), v.clone()));

        if let Some(hm) = pkgenv {
            iter.chain(hm.iter().map(|(k, v)| (k.clone(), v.clone())))
                .collect()
        } else {
            iter.collect()
        }
    }

}
