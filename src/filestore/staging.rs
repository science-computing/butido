use std::path::Path;
use std::path::PathBuf;
use anyhow::Result;
use anyhow::Error;
use indicatif::ProgressBar;
use futures::stream::Stream;
use resiter::Map;
use tar;

use crate::filestore::util::FileStoreImpl;

// The implementation of this type must be available in the merged filestore.
pub struct StagingStore(pub (in crate::filestore) FileStoreImpl);

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

                let outputs = archive.entries()?
                    .map(|ent| {
                        let p = ent?.path()?.into_owned();
                        Ok(p)
                    })
                    .map_ok(|path| dest.join(path))
                    .collect::<Result<Vec<_>>>()?;

                tar::Archive::new(&bytes[..])
                    .unpack(dest)
                    .map_err(Error::from)
                    .map(|_| outputs)
            })?
            .into_iter()
            .map(|path| {
                self.0.load_from_path(&path).map(|art| art.path().clone())
            })
            .collect()
    }
}

