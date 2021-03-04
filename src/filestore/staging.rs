//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::fmt::Debug;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use futures::stream::Stream;
use indicatif::ProgressBar;
use log::trace;
use result_inspect::ResultInspect;

use crate::filestore::path::ArtifactPath;
use crate::filestore::path::StoreRoot;
use crate::filestore::util::FileStoreImpl;

// The implementation of this type must be available in the merged filestore.
pub struct StagingStore(pub(in crate::filestore) FileStoreImpl);

impl Debug for StagingStore {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "StagingStore(root: {})", self.0.root.display())
    }
}

impl StagingStore {
    pub fn load(root: StoreRoot, progress: &ProgressBar) -> Result<Self> {
        FileStoreImpl::load(root, progress).map(StagingStore)
    }

    /// Write the passed tar stream to the file store
    ///
    /// # Returns
    ///
    /// Returns a list of Artifacts that were written from the stream
    pub async fn write_files_from_tar_stream<S>(&mut self, stream: S) -> Result<Vec<ArtifactPath>>
    where
        S: Stream<Item = Result<Vec<u8>>>,
    {
        use futures::stream::TryStreamExt;

        let dest = &self.0.root;
        stream
            .try_concat()
            .await
            .and_then(|bytes| {
                let mut archive = tar::Archive::new(&bytes[..]);

                let outputs = archive
                    .entries()
                    .context("Fetching entries from tar archive")?
                    .map(|ent| {
                        let p = ent?
                            .path()
                            .context("Getting path of TAR entry")?
                            .into_owned();
                        Ok(p)
                    })
                    .inspect(|p| trace!("Path in tar archive: {:?}", p))
                    .collect::<Result<Vec<_>>>()
                    .context("Collecting outputs of TAR archive")?;

                trace!("Unpacking archive to {}", dest.display());
                dest.unpack_archive_here(tar::Archive::new(&bytes[..]))
                    .context("Unpacking TAR")
                    .map_err(Error::from)
                    .map(|_| outputs)
            })
            .context("Concatenating the output bytestream")?
            .into_iter()
            .inspect(|p| trace!("Trying to load into staging store: {}", p.display()))
            .filter_map(|path| {
                if self.0.root.is_dir(&path) {
                    None
                } else {
                    Some({
                        // Clippy doesn't detect this properly
                        #[allow(clippy::redundant_clone)]
                        ArtifactPath::new(path.to_path_buf())
                            .inspect(|r| trace!("Loaded from path {} = {:?}", path.display(), r))
                            .with_context(|| anyhow!("Loading from path: {}", path.display()))
                            .map(|ap| self.0.load_from_path(&ap).clone())
                    })
                }
            })
            .collect()
    }

    pub fn root_path(&self) -> &StoreRoot {
        self.0.root_path()
    }

    pub fn get(&self, p: &ArtifactPath) -> Option<&ArtifactPath> {
        self.0.get(p)
    }
}
