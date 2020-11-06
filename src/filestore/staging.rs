use std::path::Path;
use std::path::PathBuf;
use std::fmt::Debug;

use anyhow::anyhow;
use anyhow::Result;
use anyhow::Error;
use anyhow::Context;
use indicatif::ProgressBar;
use futures::stream::Stream;
use resiter::Map;
use resiter::Filter;
use tar;

use crate::filestore::util::FileStoreImpl;
use crate::filestore::Artifact;

// The implementation of this type must be available in the merged filestore.
pub struct StagingStore(pub (in crate::filestore) FileStoreImpl);

impl Debug for StagingStore {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "StagingStore(root: {})", self.0.root.display())
    }
}

impl StagingStore {
    pub fn load(root: &Path, progress: ProgressBar) -> Result<Self> {
        FileStoreImpl::load(root, progress).map(StagingStore)
    }

    /// Write the passed tar stream to the file store
    ///
    /// # Returns
    ///
    /// Returns a list of Artifacts that were written from the stream
    pub async fn write_files_from_tar_stream<S>(&mut self, stream: S) -> Result<Vec<PathBuf>>
        where S: Stream<Item = Result<Vec<u8>>>
    {
        use futures::stream::TryStreamExt;
        use std::io::Read;

        let dest = &self.0.root;
        stream.try_concat()
            .await
            .and_then(|bytes| {
                let mut archive = tar::Archive::new(&bytes[..]);

                let outputs = archive.entries()
                    .context("Fetching entries from tar archive")?
                    .map(|ent| {
                        let p = ent?.path().context("Getting path of TAR entry")?.into_owned();
                        Ok(p)
                    })
                    .map_ok(|path| dest.join(path))
                    .filter_ok(|p| p.is_file())
                    .collect::<Result<Vec<_>>>()
                    .context("Collecting outputs of TAR archive")?;

                tar::Archive::new(&bytes[..])
                    .unpack(dest)
                    .context("Unpacking TAR")
                    .map_err(Error::from)
                    .map(|_| outputs)
            })
            .context("Concatenating the output bytestream")?
            .into_iter()
            .map(|path| {
                self.0.load_from_path(&path)
                    .with_context(|| anyhow!("Loading from path: {}", path.display()))
                    .map_err(Error::from)
                    .map(|art| art.path().clone())
            })
            .collect()
    }

    pub fn load_from_path(&mut self, pb: &PathBuf) -> Result<&Artifact> {
        self.0.load_from_path(pb)
    }

    pub fn root_path(&self) -> &Path {
        self.0.root_path()
    }

    pub fn path_exists_in_store_root(&self, path: &Path) -> bool {
        self.0.path_exists_in_store_root(path)
    }
}

