//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::collections::BTreeMap;

use uuid::Uuid;
use getset::Getters;

use crate::job::Job;
use crate::job::JobResource;
use crate::package::PhaseName;
use crate::package::Shebang;
use crate::util::docker::ImageName;

#[derive(Debug, Getters)]
pub struct Tree {
    #[getset(get = "pub")]
    inner: BTreeMap<Uuid, JobDefinition>,
}

impl Tree {
    pub fn from_package_tree(pt: crate::package::Tree,
        script_shebang: Shebang,
        image: ImageName,
        phases: Vec<PhaseName>,
        resources: Vec<JobResource>,
    ) -> Self {
        Tree { inner: Self::build_tree(pt, script_shebang, image, phases, resources) }
    }

    fn build_tree(pt: crate::package::Tree,
        script_shebang: Shebang,
        image: ImageName,
        phases: Vec<PhaseName>,
        resources: Vec<JobResource>,
    ) -> BTreeMap<Uuid, JobDefinition> {
        let mut tree = BTreeMap::new();

        for (package, dependencies) in pt.into_iter() {
            let mut deps = Self::build_tree(dependencies,
                script_shebang.clone(),
                image.clone(),
                phases.clone(),
                resources.clone());

            let deps_uuids = deps.keys().cloned().collect();
            tree.append(&mut deps);

            let job = Job::new(package,
                script_shebang.clone(),
                image.clone(),
                phases.clone(),
                resources.clone());

            let job_uuid = *job.uuid();
            let jdef = JobDefinition { job, dependencies: deps_uuids };

            tree.insert(job_uuid, jdef);
        }

        tree
    }

}

/// A job definition is the job itself and all UUIDs from jobs this job depends on.
#[derive(Debug)]
pub struct JobDefinition {
    pub job: Job,

    /// Uuids of the jobs where this job depends on the outputs
    pub dependencies: Vec<Uuid>,
}
