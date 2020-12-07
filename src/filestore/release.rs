use std::fmt::Debug;

use anyhow::Result;
use indicatif::ProgressBar;

use crate::filestore::util::FileStoreImpl;
use crate::filestore::path::StoreRoot;

// The implementation of this type must be available in the merged filestore.
pub struct ReleaseStore(pub (in crate::filestore) FileStoreImpl);

impl Debug for ReleaseStore {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "StagingStore(root: {})", self.0.root.display())
    }
}


impl ReleaseStore {
    pub fn load(root: StoreRoot, progress: ProgressBar) -> Result<Self> {
        FileStoreImpl::load(root, progress).map(ReleaseStore)
    }
}

