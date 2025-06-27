//
// Copyright (c) 2020-2022 science+computing ag and other contributors
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
use resiter::Filter;
use resiter::Map;
use tracing::trace;

use crate::filestore::staging::StagingStore;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreRoot(PathBuf);

impl StoreRoot {
    pub fn new(root: PathBuf) -> Result<Self> {
        if root.is_absolute() {
            if root.is_dir() {
                Ok(StoreRoot(root))
            } else if root.parent().map(Path::is_dir).unwrap_or(false) {
                tracing::info!(
                    "Creating a missing release store directory: {}",
                    root.display()
                );
                std::fs::create_dir(&root).with_context(|| {
                    anyhow!(
                        "Couldn't automatically create the following release store directory: {}",
                        root.display()
                    )
                })?;
                Ok(StoreRoot(root))
            } else {
                Err(anyhow!(
                    "The following StoreRoot path does not point to a directory: {}",
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

    pub fn join<'a>(&'a self, ap: &'a ArtifactPath) -> Result<Option<FullArtifactPath<'a>>> {
        let join = self.0.join(&ap.0);

        if join.is_file() {
            Ok(Some(FullArtifactPath(self, ap)))
        } else if join.is_dir() {
            Err(anyhow!("Cannot load non-file path: {}", join.display()))
        } else {
            Ok(None)
        }
    }

    pub(in crate::filestore) fn is_dir(&self, subpath: &Path) -> bool {
        self.0.join(subpath).is_dir()
    }

    pub fn display(&self) -> std::path::Display<'_> {
        self.0.display()
    }

    pub(in crate::filestore) fn find_artifacts_recursive(
        &self,
    ) -> impl Iterator<Item = Result<ArtifactPath>> + use<> {
        trace!("Loading artifacts from directory: {:?}", self.0);
        let root = self.0.clone();
        walkdir::WalkDir::new(&self.0)
            .follow_links(false)
            .into_iter()
            .filter_ok(|e| {
                let is_file = e.file_type().is_file();
                trace!("{:?} is file = {}", e, is_file);
                is_file
            })
            .inspect(|p| trace!("Loading Artifact from path: {:?}", p))
            .map_err(Error::from)
            .and_then_ok(move |de| {
                de.path()
                    .strip_prefix(&root)
                    .map(|p| p.to_path_buf())
                    .map_err(Error::from)
            })
            .and_then_ok(ArtifactPath::new)
    }

    /// Unpack a tar archive in this location
    ///
    /// This function unpacks the provided tar archive "butido-style" in the location pointed to by
    /// `self` and returns the written paths.
    ///
    /// The function filters out the "/output" directory (that's what is meant by "butido-style").
    pub(in crate::filestore) fn unpack_archive_here<R>(
        &self,
        mut ar: tar::Archive<R>,
    ) -> Result<Vec<PathBuf>>
    where
        R: std::io::Read,
    {
        ar.entries()?
            .map_err(Error::from)
            .filter_ok(|entry| entry.header().entry_type() == tar::EntryType::Regular)
            .and_then_ok(|mut entry| -> Result<_> {
                let path = entry
                    .path()
                    .context("Getting path from entry in Archive")?
                    .components()
                    .filter(|comp| {
                        trace!("Filtering path component: '{:?}'", comp);
                        let osstr = std::ffi::OsStr::new(crate::consts::OUTPUTS_DIR_NAME);
                        match comp {
                            std::path::Component::Normal(s) => *s != osstr,
                            _ => true,
                        }
                    })
                    .collect::<PathBuf>();

                trace!("Path = '{:?}'", path);
                let unpack_dest = self.0.join(&path);
                trace!("Unpack to = '{:?}'", unpack_dest);

                entry.unpack(unpack_dest).map(|_| path).map_err(Error::from)
            })
            .collect::<Result<Vec<_>>>()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactPath(PathBuf);

impl ArtifactPath {
    pub fn new(p: PathBuf) -> Result<Self> {
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

    pub fn display(&self) -> std::path::Display<'_> {
        self.0.display()
    }

    pub fn file_name(&self) -> Option<&OsStr> {
        self.0.file_name()
    }

    pub fn to_str(&self) -> Option<&str> {
        self.0.to_str()
    }
}

impl AsRef<Path> for ArtifactPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FullArtifactPath<'a>(&'a StoreRoot, &'a ArtifactPath);

impl<'a> FullArtifactPath<'a> {
    pub fn is_in_staging_store(&self, store: &StagingStore) -> bool {
        store.0.root_path() == self.0
    }

    pub fn artifact_path(&self) -> &ArtifactPath {
        self.1
    }

    pub fn joined(&self) -> PathBuf {
        self.0 .0.join(&self.1 .0)
    }

    pub fn display(&self) -> FullArtifactPathDisplay<'a> {
        FullArtifactPathDisplay(self.0, self.1)
    }

    pub async fn read(self) -> Result<Vec<u8>> {
        tokio::fs::read(self.joined())
            .await
            .with_context(|| anyhow!("Reading artifact from path {}", self.0.display()))
    }
}

#[derive(Debug)]
pub struct FullArtifactPathDisplay<'a>(&'a StoreRoot, &'a ArtifactPath);

impl std::fmt::Display for FullArtifactPathDisplay<'_> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}/{}", self.0.display(), self.1.display())
    }
}
