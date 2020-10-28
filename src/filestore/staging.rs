use std::path::Path;
use std::path::PathBuf;
use anyhow::Result;

use crate::filestore::util::FileStoreImpl;

// The implementation of this type must be available in the merged filestore.
pub struct StagingStore(pub (in crate::filestore) FileStoreImpl);

impl StagingStore {
    pub fn load(root: &Path) -> Result<Self> {
        FileStoreImpl::load(root).map(StagingStore)
    }
}

