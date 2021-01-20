//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use resiter::AndThen;
use resiter::Map;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreRoot(PathBuf);

impl StoreRoot {
    pub fn new(root: PathBuf) -> Result<Self> {
        if root.is_absolute() {
            if root.is_dir() {
                Ok(StoreRoot(root))
            } else {
                Err(anyhow!(
                    "StoreRoot path does not point to directory: {}",
                    root.display()
                ))
            }
        } else {
            Err(anyhow!(
                "StoreRoot path is not absolute: {}",
                root.display()
            ))
        }
    }

    /// Unchecked variant of StoreRoot::new()
    ///
    /// Because StoreRoot::new() accesses the filesystem, this method is necessary to construct an
    /// object of StoreRoot for a non-existing path, so that we can test its implementation without
    /// the need to create objects on the filesystem.
    #[cfg(test)]
    pub fn new_unchecked(root: PathBuf) -> Self {
        StoreRoot(root)
    }

    pub fn join<'a>(&'a self, ap: &'a ArtifactPath) -> Result<FullArtifactPath<'a>> {
        let join = self.0.join(&ap.0);

        if join.is_file() {
            Ok(FullArtifactPath(&self, ap))
        } else if join.is_dir() {
            Err(anyhow!("Cannot load non-file path: {}", join.display()))
        } else {
            Err(anyhow!("Path does not exist: {}", join.display()))
        }
    }

    /// Unchecked variant of StoreRoot::join()
    ///
    /// This function is needed (like StoreRoot::new_unchecked()) to perform a construction of
    /// FullArtifactPath like StoreRoot::join() in without its side effects (accessing the
    /// filesystem) for testing purposes
    #[cfg(test)]
    pub fn join_unchecked<'a>(&'a self, ap: &'a ArtifactPath) -> FullArtifactPath<'a> {
        FullArtifactPath(self, ap)
    }

    pub(in crate::filestore) fn is_file(&self, subpath: &Path) -> bool {
        self.0.join(subpath).is_file()
    }

    pub(in crate::filestore) fn is_dir(&self, subpath: &Path) -> bool {
        self.0.join(subpath).is_dir()
    }

    pub fn display(&self) -> std::path::Display {
        self.0.display()
    }

    pub(in crate::filestore) fn find_artifacts_recursive(
        &self,
    ) -> impl Iterator<Item = Result<ArtifactPath>> {
        walkdir::WalkDir::new(&self.0)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| e.file_type().is_file())
            .map_err(Error::from)
            .map_ok(|de| de.into_path())
            .and_then_ok(ArtifactPath::new)
    }

    pub(in crate::filestore) fn unpack_archive_here<R>(&self, mut ar: tar::Archive<R>) -> Result<()>
    where
        R: std::io::Read,
    {
        ar.unpack(&self.0).map_err(Error::from).map(|_| ())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArtifactPath(PathBuf);

impl ArtifactPath {
    pub(in crate::filestore) fn new(p: PathBuf) -> Result<Self> {
        if p.is_relative() {
            Ok(ArtifactPath(p))
        } else {
            Err(anyhow!("Path is not relative: {}", p.display()))
        }
    }

    /// Unchecked variant of ArtifactPath::new()
    ///
    /// Because ArtifactPath::new() accesses the filesystem, this method is necessary to construct an
    /// object of ArtifactPath for a non-existing path, so that we can test its implementation without
    /// the need to create objects on the filesystem.
    #[cfg(test)]
    pub fn new_unchecked(root: PathBuf) -> Self {
        ArtifactPath(root)
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FullArtifactPath<'a>(&'a StoreRoot, &'a ArtifactPath);

impl<'a> FullArtifactPath<'a> {
    fn joined(&self) -> PathBuf {
        self.0 .0.join(&self.1 .0)
    }

    pub fn exists(&self) -> bool {
        self.joined().exists()
    }

    pub fn display(&self) -> FullArtifactPathDisplay<'a> {
        FullArtifactPathDisplay(self.0, self.1)
    }

    pub(in crate::filestore) fn file_stem(&self) -> Option<&OsStr> {
        self.1 .0.file_stem()
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
