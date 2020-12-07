use getset::Getters;
use uuid::Uuid;

use crate::job::JobResource;
use crate::package::Package;
use crate::phase::PhaseName;
use crate::util::docker::ImageName;

/// A prepared, but not necessarily runnable, job configuration
#[derive(Debug, Getters)]
pub struct Job {
    /// A unique name for the job, not necessarily human-readable
    #[getset(get = "pub")]
    pub(in super) uuid: Uuid,

    #[getset(get = "pub")]
    pub(in super) package: Package,

    #[getset(get = "pub")]
    pub(in super) image: ImageName,


    #[getset(get = "pub")]
    pub(in super) script_shebang: String,

    #[getset(get = "pub")]
    pub(in super) script_phases: Vec<PhaseName>,

    #[getset(get = "pub")]
    pub(in super) resources: Vec<JobResource>,
}

impl Job {

    pub fn new(pkg: Package, image: ImageName, phases: Vec<PhaseName>, resources: Vec<JobResource>) -> Self {
        let uuid = Uuid::new_v4();

        Job {
            uuid,
            package: pkg,
            image,
            script_shebang: String::from("#!/bin/bash"), // TODO Dont hardcode
            script_phases: phases,
            resources,
        }

    }

}

