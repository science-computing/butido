//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use crate::filestore::ArtifactPath;
use crate::util::EnvironmentVariableName;

#[derive(Clone, Debug)]
pub enum JobResource {
    Environment(EnvironmentVariableName, String),
    Artifact(ArtifactPath),
}

impl From<(EnvironmentVariableName, String)> for JobResource {
    fn from(tpl: (EnvironmentVariableName, String)) -> Self {
        JobResource::Environment(tpl.0, tpl.1)
    }
}

impl From<ArtifactPath> for JobResource {
    fn from(a: ArtifactPath) -> Self {
        JobResource::Artifact(a)
    }
}

impl JobResource {
    pub fn env(&self) -> Option<(&EnvironmentVariableName, &String)> {
        match self {
            JobResource::Environment(k, v) => Some((k, v)),
            _ => None,
        }
    }
    pub fn artifact(&self) -> Option<&ArtifactPath> {
        match self {
            JobResource::Artifact(a) => Some(a),
            _ => None,
        }
    }
}
