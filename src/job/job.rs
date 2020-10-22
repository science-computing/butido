use uuid::Uuid;

use crate::job::JobResource;
use crate::package::Package;
use crate::phase::PhaseName;
use crate::util::docker::ImageName;

/// A prepared, but not necessarily runnable, job configuration
#[derive(Debug)]
pub struct Job {
    /// A unique name for the job, not necessarily human-readable
    pub(in super) uuid: Uuid,

    pub(in super) package: Package,
    pub(in super) image: ImageName,

    pub(in super) script_shebang: String,
    pub(in super) script_phases: Vec<PhaseName>,

    pub(in super) resources: Vec<JobResource>,
}

impl Job {

    pub fn new(pkg: Package, image: ImageName, phases: Vec<PhaseName>) -> Self {
        let uuid = Uuid::new_v4();

        Job {
            uuid,
            package: pkg,
            image,
            script_shebang: String::from("#!/bin/bash"), // TODO Dont hardcode
            script_phases: phases,
            resources: Vec::new(),
        }

    }

    pub fn add_resource(&mut self, res: JobResource) {
        self.resources.push(res)
    }

}

