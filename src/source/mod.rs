use std::path::PathBuf;

use anyhow::Result;
use anyhow::Error;
use url::Url;

use crate::package::Package;
use crate::package::PackageName;
use crate::package::PackageVersion;
use crate::package::Source;
use crate::util::progress::ProgressBars;

#[derive(Clone, Debug)]
pub struct SourceCache {
    root: PathBuf,
}

impl SourceCache {
    pub fn new(root: PathBuf) -> Self {
        SourceCache { root }
    }

    pub fn source_for(&self, p: &Package) -> SourceEntry {
        SourceEntry::for_package(self.root.clone(), p)
    }
}

#[derive(Debug)]
pub struct SourceEntry {
    cache_root: PathBuf,
    package_name: PackageName,
    package_version: PackageVersion,
    package_source: Source,
    package_source_path: PathBuf,
}

impl SourceEntry {

    fn for_package(cache_root: PathBuf, package: &Package) -> Self {
        let package_source_path = cache_root.join(format!("{}-{}.source", package.name(), package.version()));

        SourceEntry {
            cache_root,
            package_name: package.name().clone(),
            package_version: package.version().clone(),
            package_source: package.source().clone(),
            package_source_path
        }
    }

    pub fn exists(&self) -> bool {
        self.package_source_path.exists()
    }

    pub fn path(&self) -> &PathBuf {
        &self.package_source_path
    }

    pub fn url(&self) -> &Url {
        self.package_source.url()
    }

    pub async fn remove_file(&self) -> Result<()> {
        tokio::fs::remove_file(&self.package_source_path).await.map_err(Error::from)
    }

    pub async fn verify_hash(&self) -> Result<bool> {
        use tokio::io::AsyncReadExt;

        let mut buf = vec![];
        tokio::fs::OpenOptions::new()
            .create(false)
            .create_new(false)
            .read(true)
            .open(&self.package_source_path)
            .await?
            .read_to_end(&mut buf)
            .await?;

        self.package_source.hash().matches_hash_of(&buf)
    }

    pub async fn open(&self) -> Result<tokio::fs::File> {
        tokio::fs::OpenOptions::new()
            .create(false)
            .create_new(false)
            .read(true)
            .open(&self.package_source_path)
            .await
            .map_err(Error::from)
    }

    pub async fn create(&self) -> Result<tokio::fs::File> {
        tokio::fs::OpenOptions::new()
            .create(true)
            .create_new(true)
            .write(true)
            .open(&self.package_source_path)
            .await
            .map_err(Error::from)
    }

}

