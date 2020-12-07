//! Module containing utilities for the filestore implementation
//!

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use indicatif::ProgressBar;
use resiter::AndThen;
use resiter::Map;
use walkdir::WalkDir;

use crate::filestore::Artifact;
use crate::filestore::path::*;

/// The actual filestore implementation
///
/// Because the "staging" filestore and the "release" filestore function the same underneath, we
/// provide this type as the implementation.
///
/// It can then be wrapped into the actual interface of this module with specialized functionality.
pub struct FileStoreImpl {
    pub(in crate::filestore) root: StoreRoot,
    store: BTreeMap<ArtifactPath, Artifact>,
}

impl FileStoreImpl {
    /// Loads the passed path recursively into a Path => Artifact mapping
    pub fn load(path: &Path, progress: ProgressBar) -> Result<Self> {
        if path.is_dir() {
            let root = StoreRoot::new(path.to_path_buf());

            let store = WalkDir::new(&path)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| e.file_type().is_file())
                .map_err(Error::from)
                .and_then_ok(|f| {
                    progress.tick();
                    let p = root.stripped_from(f.path())?;
                    Artifact::load(&root, p.clone()).map(|a| (p, a))
                })
                .collect::<Result<BTreeMap<ArtifactPath, Artifact>>>()?;

            Ok(FileStoreImpl { root, store })
        } else {
            Err(anyhow!("File store cannot be loaded from non-directory: {}", path.display()))
        }
    }

    pub fn root_path(&self) -> &StoreRoot {
        &self.root
    }

    pub fn path_exists_in_store_root(&self, p: &Path) -> bool {
        self.root.join_path(p).is_file()
    }

    pub (in crate::filestore) fn values(&self) -> impl Iterator<Item = &Artifact> {
        self.store.values()
    }

    pub (in crate::filestore) fn load_from_path(&mut self, artifact_path: &ArtifactPath) -> Result<&Artifact> {
        if self.store.get(&artifact_path).is_some() {
            Err(anyhow!("Entry exists: {}", artifact_path.display()))
        } else {
            Ok(self.store.entry(artifact_path.clone()).or_insert(Artifact::load(&self.root, artifact_path.clone())?))
        }
    }

}

