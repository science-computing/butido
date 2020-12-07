//! Module containing utilities for the filestore implementation
//!

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use indicatif::ProgressBar;
use resiter::AndThen;
use resiter::Map;
use walkdir::WalkDir;

use crate::filestore::Artifact;

/// The actual filestore implementation
///
/// Because the "staging" filestore and the "release" filestore function the same underneath, we
/// provide this type as the implementation.
///
/// It can then be wrapped into the actual interface of this module with specialized functionality.
pub struct FileStoreImpl {
    pub(in crate::filestore) root: PathBuf,
    store: BTreeMap<PathBuf, Artifact>,
}

impl FileStoreImpl {
    /// Loads the passed path recursively into a Path => Artifact mapping
    pub fn load(root: &Path, progress: ProgressBar) -> Result<Self> {
        if root.is_dir() {
            let store = WalkDir::new(root)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| e.file_type().is_file())
                .map_err(Error::from)
                .map_ok(|f| f.path().to_path_buf())
                .and_then_ok(|pb| {
                    progress.tick();
                    let p = pb.strip_prefix(root)?;
                    Artifact::load(root, &p).map(|a| (pb, a))
                })
                .collect::<Result<BTreeMap<PathBuf, Artifact>>>()?;

            Ok(FileStoreImpl { root: root.to_path_buf(), store })
        } else {
            Err(anyhow!("File store cannot be loaded from non-directory: {}", root.display()))
        }
    }

    pub fn root_path(&self) -> &Path {
        &self.root
    }

    pub fn path_exists_in_store_root(&self, p: &Path) -> bool {
        self.root.join(p).is_file()
    }

    pub (in crate::filestore) fn values(&self) -> impl Iterator<Item = &Artifact> {
        self.store.values()
    }

    pub (in crate::filestore) fn load_from_path(&mut self, pb: &PathBuf) -> Result<&Artifact> {
        if !self.is_sub_path(pb)? {
            Err(anyhow!("Not a sub-path of {}: {}", self.root.display(), pb.display()))
        } else {
            if self.store.get(pb).is_some() {
                Err(anyhow!("Entry exists: {}", pb.display()))
            } else {
                let p = pb.strip_prefix(&self.root)?;
                Ok(self.store.entry(pb.to_path_buf()).or_insert(Artifact::load(&self.root, p)?))
            }
        }
    }

    fn is_sub_path(&self, p: &Path) -> Result<bool> {
        p.canonicalize()
            .map(|c| c.starts_with(&self.root))
            .map_err(Error::from)
    }

}

