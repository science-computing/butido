use crate::filestore::Artifact;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::package::PackageName;
use crate::package::PackageVersion;

/// A type that merges the release store and the staging store
///
/// The stores are not actually merged (on disk or in memory), but the querying mechanism works in
/// a way where it _always_ preferes the staging store over the release store.
///
pub struct MergedStores<'a> {
    release: &'a ReleaseStore,
    staging: &'a StagingStore,
}

impl<'a> MergedStores<'a> {
    pub (in crate::filestore) fn new(release: &'a ReleaseStore, staging: &'a StagingStore) -> Self {
        MergedStores { release, staging }
    }

    pub fn get_artifact_by_name(&self, name: &PackageName) -> Vec<&Artifact> {
        let v = self.staging.0
            .values()
            .filter(|a| a.name() == name)
            .collect::<Vec<_>>();

        if v.is_empty() {
            self.release.0
                .values()
                .filter(|a| a.name() == name)
                .collect()
        } else {
            v
        }
    }

    pub fn get_artifact_by_name_and_version(&self, name: &PackageName, version: &PackageVersion) -> Vec<&Artifact> {
        let v = self.staging.0
            .values()
            .filter(|a| a.name() == name && a.version() == version)
            .collect::<Vec<_>>();

        if v.is_empty() {
            self.release.0
                .values()
                .filter(|a| a.name() == name && a.version() == version)
                .collect()
        } else {
            v
        }
    }

}
