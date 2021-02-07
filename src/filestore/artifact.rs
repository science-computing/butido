//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::Result;
use getset::Getters;

use crate::filestore::path::ArtifactPath;
use crate::filestore::path::StoreRoot;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Getters)]
pub struct Artifact {
    #[getset(get = "pub")]
    path: ArtifactPath,
}

impl Artifact {
    pub fn load(_root: &StoreRoot, path: ArtifactPath) -> Result<Self> {
        Ok(Artifact {
            path,
        })
    }
}

