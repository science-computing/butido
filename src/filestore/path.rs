use std::path::Path;
use std::path::PathBuf;
use std::ffi::OsStr;

use anyhow::anyhow;
use anyhow::Error;
use anyhow::Result;
use anyhow::Context;

#[derive(Clone, Debug, PartialEq, Eq)]
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

    pub fn join<'a>(&'a self, ap: &'a ArtifactPath) -> FullArtifactPath<'a> {
        FullArtifactPath(&self, ap)
    }

    pub fn is_file(&self, subpath: &Path) -> bool {
        self.0.join(subpath).is_file()
    }

    pub fn is_dir(&self, subpath: &Path) -> bool {
        self.0.join(subpath).is_dir()
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


#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
pub struct FullArtifactPath<'a>(&'a StoreRoot, &'a ArtifactPath);

impl<'a> FullArtifactPath<'a> {
    fn joined(&self) -> PathBuf {
        self.0.0.join(&self.1.0)
    }

    pub (in crate::filestore) fn is_file(&self) -> bool {
        self.joined().is_file()
    }

    pub fn display(&self) -> FullArtifactPathDisplay<'a> {
        FullArtifactPathDisplay(self.0, self.1)
    }

    pub async fn read(self) -> Result<Vec<u8>> {
        tokio::fs::read(self.joined())
            .await
            .map(Vec::from)
            .with_context(|| anyhow!("Reading artifact from path {}", self.0.display()))
            .map_err(Error::from)
    }
}

pub struct FullArtifactPathDisplay<'a>(&'a StoreRoot, &'a ArtifactPath);

impl<'a> std::fmt::Display for FullArtifactPathDisplay<'a> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}/{}", self.0.display(), self.1.display())
    }
}

