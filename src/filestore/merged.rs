//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::sync::Arc;

use log::trace;
use tokio::sync::RwLock;

use anyhow::Result;
use getset::Getters;

use crate::filestore::Artifact;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;

/// A type that merges the release store and the staging store
///
/// The stores are not actually merged (on disk or in memory), but the querying mechanism works in
/// a way where it _always_ preferes the staging store over the release store.
///
#[derive(Getters)]
pub struct MergedStores {
    release: Arc<RwLock<ReleaseStore>>,

    #[getset(get = "pub")]
    staging: Arc<RwLock<StagingStore>>,
}

impl MergedStores {
    pub fn new(release: Arc<RwLock<ReleaseStore>>, staging: Arc<RwLock<StagingStore>>) -> Self {
        MergedStores { release, staging }
    }

    pub async fn get_artifact_by_name_and_version(&self, name: &PackageName, version: &PackageVersionConstraint) -> Result<Vec<Artifact>> {
        let v = self.staging
            .read()
            .await
            .0
            .values()
            .filter(|a| {
                trace!("Checking {:?} == {:?} && {:?} == {:?}", a.name(), name, version, a.version());
                a.name() == name && version.matches(a.version())
            })
            .cloned()
            .collect::<Vec<_>>();

        if v.is_empty() {
            Ok({
                self.release
                    .read()
                    .await
                    .0
                    .values()
                    .filter(|a| a.name() == name && version.matches(a.version()))
                    .cloned()
                    .collect()
            })
        } else {
            Ok(v)
        }
    }

}
