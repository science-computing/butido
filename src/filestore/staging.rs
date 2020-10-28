use std::path::Path;
use std::path::PathBuf;
use anyhow::Result;
use indicatif::ProgressBar;

use crate::filestore::util::FileStoreImpl;

// The implementation of this type must be available in the merged filestore.
pub struct StagingStore(pub (in crate::filestore) FileStoreImpl);

impl StagingStore {
    pub fn load(root: &Path, progress: ProgressBar) -> Result<Self> {
        FileStoreImpl::load(root, progress).map(StagingStore)
    }
}

