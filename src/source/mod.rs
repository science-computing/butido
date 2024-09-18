//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use getset::Getters;
use tracing::trace;
use url::Url;

use crate::package::Package;
use crate::package::PackageName;
use crate::package::PackageVersion;
use crate::package::Source;

#[derive(Clone, Debug)]
pub struct SourceCache {
    root: PathBuf,
}

impl SourceCache {
    pub fn new(root: PathBuf) -> Self {
        SourceCache { root }
    }

    pub fn sources_for(&self, p: &Package) -> Vec<SourceEntry> {
        SourceEntry::for_package(self.root.clone(), p)
    }
}

#[derive(Debug, Getters)]
pub struct SourceEntry {
    cache_root: PathBuf,
    #[getset(get = "pub")]
    package_name: PackageName,
    #[getset(get = "pub")]
    package_version: PackageVersion,
    #[getset(get = "pub")]
    package_source_name: String,
    package_source: Source,
}

impl SourceEntry {
    fn source_file_directory(&self) -> PathBuf {
        self.cache_root
            .join(format!("{}-{}", self.package_name, self.package_version))
    }

    fn for_package(cache_root: PathBuf, package: &Package) -> Vec<Self> {
        package
            .sources()
            .clone()
            .into_iter()
            .map(|(source_name, source)| SourceEntry {
                cache_root: cache_root.clone(),
                package_name: package.name().clone(),
                package_version: package.version().clone(),
                package_source_name: source_name,
                package_source: source,
            })
            .collect()
    }

    pub fn path(&self) -> PathBuf {
        self.source_file_directory().join({
            (self.package_source_name.as_ref() as &std::path::Path).with_extension("source")
        })
    }

    pub fn path_as_string(&self) -> String {
        self.path().to_string_lossy().to_string()
    }

    pub fn url(&self) -> &Url {
        self.package_source.url()
    }

    pub fn download_manually(&self) -> bool {
        *self.package_source.download_manually()
    }

    pub async fn remove_file(&self) -> Result<()> {
        let p = self.path();
        tokio::fs::remove_file(&p).await?;
        Ok(())
    }

    pub async fn verify_hash(&self) -> Result<()> {
        let p = self.path();
        trace!("Verifying : {}", p.display());

        let reader = tokio::fs::OpenOptions::new()
            .create(false)
            .create_new(false)
            .read(true)
            .open(&p)
            .await
            .map(tokio::io::BufReader::new)
            .context("Opening file failed")?;

        trace!("Reader constructed for path: {}", p.display());
        self.package_source.hash().matches_hash_of(reader).await
    }

    pub async fn create(&self) -> Result<tokio::fs::File> {
        let p = self.path();
        trace!("Creating source file: {}", p.display());

        if !self.cache_root.is_dir() {
            trace!("Cache root does not exist: {}", self.cache_root.display());
            return Err(anyhow!(
                "Cache root {} does not exist!",
                self.cache_root.display()
            ));
        }

        {
            let dir = self.source_file_directory();
            if !dir.is_dir() {
                trace!("Creating directory: {}", dir.display());
                tokio::fs::create_dir_all(&dir).await.with_context(|| {
                    anyhow!(
                        "Creating source cache directory for package {} {}: {}",
                        self.package_source_name,
                        self.package_source.hash().value(),
                        dir.display()
                    )
                })?;
            } else {
                trace!("Directory exists: {}", dir.display());
            }
        }

        trace!("Creating file now: {}", p.display());
        tokio::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&p)
            .await
            .with_context(|| anyhow!("Creating file: {}", p.display()))
            .map_err(Error::from)
    }
}
