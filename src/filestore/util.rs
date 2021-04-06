//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Module containing utilities for the filestore implementation
//!

use std::collections::HashSet;

use anyhow::Result;
use indicatif::ProgressBar;

use crate::filestore::path::ArtifactPath;
use crate::filestore::path::StoreRoot;

/// The actual filestore implementation
///
/// Because the "staging" filestore and the "release" filestore function the same underneath, we
/// provide this type as the implementation.
///
/// It can then be wrapped into the actual interface of this module with specialized functionality.
#[derive(getset::Getters)]
pub struct FileStoreImpl {
    #[getset(get = "pub")]
    root_path: StoreRoot,
    store: HashSet<ArtifactPath>,
}

impl FileStoreImpl {
    /// Loads the passed path recursively
    pub fn load(root_path: StoreRoot, progress: &ProgressBar) -> Result<Self> {
        let store = root_path
            .find_artifacts_recursive()
            .inspect(|path| {
                log::trace!("Found artifact path: {:?}", path);
                progress.tick();
            })
            .collect::<Result<HashSet<ArtifactPath>>>()?;

        Ok(FileStoreImpl { root_path, store })
    }

    pub fn get(&self, artifact_path: &ArtifactPath) -> Option<&ArtifactPath> {
        self.store.get(artifact_path)
    }

    pub(in crate::filestore) fn load_from_path<'a>(
        &mut self,
        artifact_path: &'a ArtifactPath,
    ) -> &'a ArtifactPath {
        if !self.store.contains(artifact_path) {
            self.store.insert(artifact_path.clone());
        }
        artifact_path
    }
}
