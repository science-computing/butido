use std::sync::Arc;
use std::sync::RwLock;

use anyhow::anyhow;
use anyhow::Result;

use crate::filestore::Artifact;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::package::PackageName;
use crate::package::PackageVersion;
use crate::package::PackageVersionConstraint;

/// A type that merges the release store and the staging store
///
/// The stores are not actually merged (on disk or in memory), but the querying mechanism works in
/// a way where it _always_ preferes the staging store over the release store.
///
pub struct MergedStores {
    release: Arc<RwLock<ReleaseStore>>,
    staging: Arc<RwLock<StagingStore>>,
}

impl MergedStores {
    pub fn new(release: Arc<RwLock<ReleaseStore>>, staging: Arc<RwLock<StagingStore>>) -> Self {
        MergedStores { release, staging }
    }

    pub fn get_artifact_by_name(&self, name: &PackageName) -> Result<Vec<&Artifact>> {
        let v = self.staging
            .read()
            .map_err(|_| anyhow!("Lock poisoned"))
            .map(|s| {
                s.0.values()
                    .filter(|a| a.name() == name)
                    .collect::<Vec<_>>()
            })?;

        if v.is_empty() {
            self.release
                .read()
                .map_err(|_| anyhow!("Lock poisoned"))
                .map(|r| {
                    r.0.values()
                        .filter(|a| a.name() == name)
                        .collect()
                })
        } else {
            Ok(v)
        }
    }

    pub fn get_artifact_by_name_and_version(&self, name: &PackageName, version: &PackageVersionConstraint) -> Result<Vec<&Artifact>> {
        let v = self.staging
            .read()
            .map_err(|_| anyhow!("Lock poisoned"))
            .map(|s| {
                s.0.values()
                    .filter(|a| a.name() == name && version.matches(a.version()))
                    .collect::<Vec<_>>()
            })?;

        if v.is_empty() {
            self.release
                .read()
                .map_err(|_| anyhow!("Lock poisoned"))
                .map(|r| {
                    r.0.values()
                        .filter(|a| a.name() == name && version.matches(a.version()))
                        .collect()
                })
        } else {
            Ok(v)
        }
    }

}
