//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use daggy::Dag as DaggyDag;
use daggy::Walker;
use getset::Getters;
use uuid::Uuid;

use crate::job::Job;
use crate::job::JobResource;
use crate::package::Package;
use crate::package::PhaseName;
use crate::package::Shebang;
use crate::util::docker::ImageName;

#[derive(Debug, Getters)]
pub struct Dag {
    #[getset(get = "pub")]
    dag: DaggyDag<Job, i8>,
}

impl Dag {
    pub fn from_package_dag(
        dag: crate::package::Dag,
        script_shebang: Shebang,
        image: ImageName,
        phases: Vec<PhaseName>,
        resources: Vec<JobResource>,
    ) -> Self {
        let build_job = |_, p: &Package| {
            Job::new(
                p.clone(),
                script_shebang.clone(),
                image.clone(),
                phases.clone(),
                resources.clone(),
            )
        };

        Dag {
            dag: dag.dag().map(build_job, |_, e| *e),
        }
    }

    pub fn iter(&'_ self) -> impl Iterator<Item = JobDefinition> + '_ {
        self.dag.graph().node_indices().map(move |idx| {
            let job = self.dag.graph().node_weight(idx).unwrap(); // TODO
            let children = self.dag.children(idx);
            let children_uuids = children
                .iter(&self.dag)
                .filter_map(|(_, node_idx)| self.dag.graph().node_weight(node_idx))
                .map(Job::uuid)
                .cloned()
                .collect();

            JobDefinition {
                job,
                dependencies: children_uuids,
            }
        })
    }
}

#[derive(Debug)]
pub struct JobDefinition<'a> {
    pub job: &'a Job,
    pub dependencies: Vec<Uuid>,
}
