//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use getset::Getters;
use uuid::Uuid;

use crate::job::JobResource;
use crate::package::Package;
use crate::package::PhaseName;
use crate::package::Shebang;
use crate::util::docker::ImageName;

/// A prepared, but not necessarily runnable, job configuration
#[derive(Debug, Getters)]
pub struct Job {
    /// A unique name for the job, not necessarily human-readable
    #[getset(get = "pub")]
    pub(super) uuid: Uuid,

    #[getset(get = "pub")]
    pub(super) package: Package,

    #[getset(get = "pub")]
    pub(super) image: ImageName,

    #[getset(get = "pub")]
    pub(super) script_shebang: Shebang,

    #[getset(get = "pub")]
    pub(super) script_phases: Vec<PhaseName>,

    #[getset(get = "pub")]
    pub(super) resources: Vec<JobResource>,
}

impl Job {
    pub fn new(
        pkg: Package,
        script_shebang: Shebang,
        image: ImageName,
        phases: Vec<PhaseName>,
        resources: Vec<JobResource>,
    ) -> Self {
        let uuid = Uuid::new_v4();

        Job {
            uuid,
            package: pkg,
            image,
            script_shebang,
            script_phases: phases,
            resources,
        }
    }
}
