use getset::Getters;
use uuid::Uuid;
use anyhow::Result;

use crate::job::Job;
use crate::job::JobResource;
use crate::package::Package;
use crate::package::Script;
use crate::package::ScriptBuilder;
use crate::phase::PhaseName;
use crate::util::docker::ImageName;

/// A job configuration that can be run. All inputs are clear here.
#[derive(Debug, Getters)]
pub struct RunnableJob {
    uuid: Uuid,

    #[getset(get = "pub")]
    package:  Package,

    #[getset(get = "pub")]
    image:    ImageName,

    #[getset(get = "pub")]
    script:   Script,

    #[getset(get = "pub")]
    resources: Vec<JobResource>,
}

impl RunnableJob {
    pub fn build_from_job(job: Job) -> Result<Self> {
        let script = ScriptBuilder::new(&job.script_shebang)
            .build(&job.package, &job.script_phases)?;

        Ok(RunnableJob {
            uuid: job.uuid,
            package: job.package,
            image: job.image,
            resources: job.resources,

            script,
        })

    }
}
