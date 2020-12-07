use std::path::Path;
use std::path::PathBuf;
use std::ffi::OsStr;

use anyhow::Error;
use anyhow::Result;

#[derive(Debug)]
pub struct StoreRoot(PathBuf);

impl StoreRoot {
    pub (in crate::filestore) fn new(root: PathBuf) -> Self {
        StoreRoot(root)
    }

    pub (in crate::filestore) fn stripped_from(&self, pb: &Path) -> Result<ArtifactPath> {
        pb.strip_prefix(&self.0)
            .map(|p| ArtifactPath::new(p.to_path_buf()))
            .map_err(Error::from)
    }

    pub fn join(&self, ap: &ArtifactPath) -> FullArtifactPath {
        let join = self.0.join(&ap.0);
        FullArtifactPath(join)
    }

    // Needed for FileStoreImpl::path_exists_in_store_root()
    pub (in crate::filestore) fn join_path(&self, p: &Path) -> PathBuf {
        self.0.join(p)
    }

    pub fn display(&self) -> std::path::Display {
        self.0.display()
    }
}

impl AsRef<Path> for StoreRoot {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactPath(PathBuf);

impl ArtifactPath {
    pub (in crate::filestore) fn new(p: PathBuf) -> Self {
        ArtifactPath(p)
    }

    pub fn display(&self) -> std::path::Display {
        self.0.display()
    }

    pub fn file_name(&self) -> Option<&OsStr> {
        self.0.file_name()
    }

    pub fn to_str(&self) -> Option<&str> {
        self.0.to_str()
    }

    pub (in crate::filestore) fn file_stem(&self) -> Option<&OsStr> {
        self.0.file_stem()
    }

    pub (in crate::filestore) fn is_dir(&self) -> bool {
        self.0.is_dir()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FullArtifactPath(PathBuf);

impl AsRef<Path> for FullArtifactPath {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

impl FullArtifactPath {
    pub (in crate::filestore) fn is_dir(&self) -> bool {
        self.0.is_dir()
    }

    pub (in crate::filestore) fn is_file(&self) -> bool {
        self.0.is_file()
    }

    pub fn display(&self) -> std::path::Display {
        self.0.display()
    }
}

