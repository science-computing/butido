use std::path::Path;
use std::path::PathBuf;
use anyhow::Result;

use crate::filestore::util::FileStoreImpl;
use crate::filestore::MergedStores;
use crate::filestore::StagingStore;

// The implementation of this type must be available in the merged filestore.
pub struct ReleaseStore(pub (in crate::filestore) FileStoreImpl);

impl ReleaseStore {
    pub fn load(root: &Path) -> Result<Self> {
        FileStoreImpl::load(root).map(ReleaseStore)
    }

    pub fn merged<'a>(&'a self, staging: &'a StagingStore) -> MergedStores<'a> {
        MergedStores::new(self, staging)
    }
}

