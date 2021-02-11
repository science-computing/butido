//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

// TODO: The MergedStores is not used at all anymore, because we removed the feature while doing
// the rewrite
#![allow(unused)]


use std::sync::Arc;
use std::path::Path;

use anyhow::Result;
use getset::Getters;
use log::trace;
use tokio::sync::RwLock;

use crate::filestore::path::ArtifactPath;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;


/// A type that merges the release store and the staging store
///
/// The stores are not actually merged (on disk or in memory), but the querying mechanism works in
/// a way where it _always_ preferes the staging store over the release store.
///
#[derive(Getters)]
pub struct MergedStores {
    #[getset(get = "pub")]
    release: Arc<RwLock<ReleaseStore>>,

    #[getset(get = "pub")]
    staging: Arc<RwLock<StagingStore>>,
}

impl MergedStores {
    pub fn new(release: Arc<RwLock<ReleaseStore>>, staging: Arc<RwLock<StagingStore>>) -> Self {
        MergedStores { release, staging }
    }

    pub async fn get_artifact_by_path(&self, p: &Path) -> Result<Option<ArtifactPath>> {
        trace!("Fetching artifact from path: {:?}", p.display());
        let artifact_path = ArtifactPath::new(p.to_path_buf())?;

        let staging = &mut self.staging.write().await.0;
        let staging_path = staging.root_path().join(&artifact_path)?;
        trace!("staging_path = {:?}", staging_path.display());

        if staging_path.exists() {
            let art = if let Some(art) = staging.get(&artifact_path) {
                art
            } else {
                trace!("Loading path from staging store: {:?}", artifact_path.display());
                staging.load_from_path(&artifact_path)
            };

            return Ok(Some(art.clone()))
        }

        let release = &mut self.release.write().await.0;
        let release_path = release.root_path().join(&artifact_path)?;
        trace!("release_path = {:?}", release_path);

        if release_path.exists() {
            let art = if let Some(art) = release.get(&artifact_path) {
                art
            } else {
                trace!("Loading path from release store: {:?}", artifact_path.display());
                release.load_from_path(&artifact_path)
            };
            return Ok(Some(art.clone()))
        }

        Ok(None)
    }

    pub async fn get(&self, p: &ArtifactPath) -> Option<ArtifactPath> {
        if let Some(a) = self.staging.read().await.get(p).cloned() {
            return Some(a)
        }

        self.release.read().await.get(p).cloned()
    }
}
