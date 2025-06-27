//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use getset::Getters;
use petgraph::acyclic::Acyclic;
use petgraph::graph::DiGraph;
use uuid::Uuid;

use crate::job::Job;
use crate::job::JobResource;
use crate::package::DependencyType;
use crate::package::Package;
use crate::package::PhaseName;
use crate::package::Shebang;
use crate::util::docker::ImageName;

#[derive(Debug, Getters)]
pub struct Dag {
    #[getset(get = "pub")]
    dag: Acyclic<DiGraph<Job, DependencyType>>,
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
            dag: Acyclic::<_>::try_from_graph(dag.dag().map(build_job, |_, e| (*e).clone()))
                .unwrap(), // The dag.dag() is already acyclic so this cannot fail
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = JobDefinition<'_>> {
        self.dag.node_indices().map(move |idx| {
            let job = self.dag.node_weight(idx).unwrap(); // TODO
            let children = self.dag.neighbors_directed(idx, petgraph::Outgoing);
            let children_uuids = children
                .filter_map(|node_idx| self.dag.node_weight(node_idx))
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
