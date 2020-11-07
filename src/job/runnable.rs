use getset::Getters;
use uuid::Uuid;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use resiter::Map;

use crate::job::Job;
use crate::job::JobResource;
use crate::package::Package;
use crate::package::Script;
use crate::package::ScriptBuilder;
use crate::phase::PhaseName;
use crate::util::docker::ImageName;
use crate::filestore::MergedStores;
use crate::package::ParseDependency;

/// A job configuration that can be run. All inputs are clear here.
#[derive(Debug, Getters)]
pub struct RunnableJob {
    #[getset(get = "pub")]
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
    pub fn build_from_job(job: Job, merged_stores: &MergedStores) -> Result<Self> {
        let script = ScriptBuilder::new(&job.script_shebang)
            .build(&job.package, &job.script_phases)?;

        trace!("Preparing build dependencies");
        let build_resources = job.package()
            .dependencies()
            .build()
            .into_iter()
            .map(|dep| Self::build_resource(dep, merged_stores));

        trace!("Preparing runtime dependencies");
        let runtime_resources = job.package()
            .dependencies()
            .runtime()
            .into_iter()
            .map(|dep| Self::build_resource(dep, merged_stores));

        let resources = build_resources.chain(runtime_resources)
            .collect::<Result<Vec<JobResource>>>()?;

        Ok(RunnableJob {
            uuid: job.uuid,
            package: job.package,
            image: job.image,
            resources: job.resources.into_iter().chain(resources.into_iter()).collect(),

            script,
        })

    }

    fn build_resource(dep: &dyn ParseDependency, merged_stores: &MergedStores) -> Result<JobResource> {
        let (name, vers) = dep.parse_as_name_and_version()?;
        trace!("Copying dep: {:?} {:?}", name, vers);
        let mut a = merged_stores.get_artifact_by_name_and_version(&name, &vers)?;

        if a.is_empty() {
            Err(anyhow!("Cannot find dependency: {:?} {:?}", name, vers))
                .context("Building a runnable job")
                .map_err(Error::from)
        } else {
            a.sort();
            let a_len = a.len();
            let found_dependency = a.into_iter().next().unwrap(); // save by above check
            if a_len > 1 {
                warn!("Found more than one dependency for {:?} {:?}", name, vers);
                warn!("Using: {:?}", found_dependency);
                warn!("Please investigate, this might be a BUG");
            }

            Ok(JobResource::Artifact(found_dependency.clone()))
        }

    }
}
