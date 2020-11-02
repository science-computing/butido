//! Module containing utilities for the filestore implementation
//!

use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Result;
use anyhow::Error;
use walkdir::WalkDir;
use resiter::Map;
use resiter::AndThen;
use indicatif::ProgressBar;

use crate::filestore::Artifact;
use crate::package::PackageName;
use crate::package::PackageVersion;

/// The actual filestore implementation
///
/// Because the "staging" filestore and the "release" filestore function the same underneath, we
/// provide this type as the implementation.
///
/// It can then be wrapped into the actual interface of this module with specialized functionality.
pub struct FileStoreImpl {
    root: PathBuf,
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
                    Artifact::load(&pb).map(|a| (pb, a))
                })
                .collect::<Result<BTreeMap<PathBuf, Artifact>>>()?;

            Ok(FileStoreImpl { root: root.to_path_buf(), store })
        } else {
            Err(anyhow!("File store cannot be loaded from non-directory: {}", root.display()))
        }
    }

    pub (in crate::filestore) fn values(&self) -> impl Iterator<Item = &Artifact> {
        self.store.values()
    }

    pub fn get(&self, p: &Path) -> Option<&Artifact> {
        self.store.get(p)
    }

    pub fn get_artifact_by_name(&self, name: &PackageName) -> Vec<&Artifact> {
        self.store
            .values()
            .filter(|a| a.name() == name)
            .collect()
    }

    pub fn get_artifact_by_name_and_version(&self, name: &PackageName, version: &PackageVersion) -> Vec<&Artifact> {
        self.store
            .values()
            .filter(|a| a.name() == name && a.version() == version)
            .collect()
    }

}

