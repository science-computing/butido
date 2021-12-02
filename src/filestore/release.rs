//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::fmt::Debug;

use anyhow::Result;
use indicatif::ProgressBar;

use crate::filestore::path::ArtifactPath;
use crate::filestore::path::StoreRoot;
use crate::filestore::util::FileStoreImpl;

// The implementation of this type must be available in the merged filestore.
pub struct ReleaseStore(pub(in crate::filestore) FileStoreImpl);

impl Debug for ReleaseStore {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "ReleaseStore(root: {})", self.0.root_path().display())
    }
}

impl ReleaseStore {
    pub fn load(root: StoreRoot, progress: &ProgressBar) -> Result<Self> {
        FileStoreImpl::load(root, progress).map(ReleaseStore)
    }

    pub fn root_path(&self) -> &StoreRoot {
        self.0.root_path()
    }

    pub fn get(&self, p: &ArtifactPath) -> Option<&ArtifactPath> {
        self.0.get(p)
    }
}
