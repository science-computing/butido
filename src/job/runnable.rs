//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use getset::Getters;
use tracing::{debug, trace};
use uuid::Uuid;

use crate::config::Configuration;
use crate::filestore::ArtifactPath;
use crate::job::Job;
use crate::job::JobResource;
use crate::package::Package;
use crate::package::Script;
use crate::package::ScriptBuilder;
use crate::source::SourceCache;
use crate::source::SourceEntry;
use crate::util::docker::ImageName;
use crate::util::EnvironmentVariableName;

/// A job configuration that can be run. All inputs are clear here.
#[derive(Clone, Debug, Getters)]
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
    pub fn build_from_job(
        job: &Job,
        source_cache: &SourceCache,
        config: &Configuration,
        git_author_env: Option<&(EnvironmentVariableName, String)>,
        git_commit_env: Option<&(EnvironmentVariableName, String)>,
        dependencies: Vec<ArtifactPath>,
    ) -> Result<Self> {
        if config.containers().check_env_names() {
            debug!("Checking environment if all variables are allowed!");
            job.resources()
                .iter()
                .filter_map(|r| r.env())
                .chain({
                    job.package()
                        .environment()
                        .as_ref()
                        .map(|hm| hm.iter())
                        .into_iter()
                        .flatten()
                })
                .chain(git_author_env.as_ref().into_iter().map(|(k, v)| (k, v)))
                .chain(git_commit_env.as_ref().into_iter().map(|(k, v)| (k, v)))
                .inspect(|(name, _)| debug!("Checking: {}", name))
                .try_for_each(|(name, _)| {
                    trace!(
                        "{:?} contains? {:?}",
                        config.containers().allowed_env(),
                        name
                    );
                    if !config.containers().allowed_env().contains(name) {
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

        let resources = dependencies
            .into_iter()
            .map(JobResource::from)
            .chain({
                job.resources()
                    .iter()
                    .filter(|jr| jr.env().is_some())
                    .cloned()
            })
            .chain(git_author_env.into_iter().cloned().map(JobResource::from))
            .chain(git_commit_env.into_iter().cloned().map(JobResource::from))
            .collect();

        debug!("Building script now");
        let script = ScriptBuilder::new(job.script_shebang()).build(
            job.package(),
            job.script_phases(),
            *config.strict_script_interpolation(),
        )?;

        Ok(RunnableJob {
            uuid: *job.uuid(),
            package: job.package().clone(),
            image: job.image().clone(),
            resources,
            source_cache: source_cache.clone(),

            script,
        })
    }

    pub fn package_sources(&self) -> Vec<SourceEntry> {
        self.source_cache.sources_for(self.package())
    }

    pub fn environment(&self) -> impl Iterator<Item = (&EnvironmentVariableName, &String)> {
        self.resources.iter().filter_map(|r| r.env()).chain({
            self.package()
                .environment()
                .as_ref()
                .map(|hm| hm.iter())
                .into_iter()
                .flatten()
        })
    }
}
