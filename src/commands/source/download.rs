//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::convert::TryFrom;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use clap::ArgMatches;
use log::{debug, trace};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::config::*;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;
use crate::source::*;
use crate::util::progress::ProgressBars;


#[derive(Clone)]
struct ProgressWrapper {
    download_count: u64,
    finished_downloads: u64,
    current_bytes: usize,
    sum_bytes: u64,
    bar: Arc<Mutex<indicatif::ProgressBar>>,
}

impl ProgressWrapper {
    fn new(bar: indicatif::ProgressBar) -> Self {
        Self {
            download_count: 0,
            finished_downloads: 0,
            current_bytes: 0,
            sum_bytes: 0,
            bar: Arc::new(Mutex::new(bar))
        }
    }

    async fn inc_download_count(&mut self) {
        self.download_count += 1;
        self.set_message().await;
    }

    async fn inc_download_bytes(&mut self, bytes: u64) {
        self.sum_bytes += bytes;
        self.set_message().await;
    }

    async fn finish_one_download(&mut self) {
        self.finished_downloads += 1;
        self.bar.lock().await.inc(1);
        self.set_message().await;
    }

    async fn add_bytes(&mut self, len: usize) {
        self.current_bytes += len;
        self.set_message().await;
    }

    async fn set_message(&self) {
        let bar = self.bar.lock().await;
        bar.set_message(format!("Downloading ({current_bytes}/{sum_bytes} bytes, {dlfinished}/{dlsum} downloads finished",
                current_bytes = self.current_bytes,
                sum_bytes = self.sum_bytes,
                dlfinished = self.finished_downloads,
                dlsum = self.download_count));
    }

    async fn success(&self) {
        let bar = self.bar.lock().await;
        bar.finish_with_message(format!("Succeeded {}/{} downloads", self.finished_downloads, self.download_count));
    }

    async fn error(&self) {
        let bar = self.bar.lock().await;
        bar.finish_with_message(format!("At least one download of {} failed", self.download_count));
    }
}

async fn perform_download(source: &SourceEntry, progress: Arc<Mutex<ProgressWrapper>>, timeout: Option<u64>) -> Result<()> {
    trace!("Creating: {:?}", source);
    let file = source.create().await.with_context(|| {
        anyhow!(
            "Creating source file destination: {}",
            source.path().display()
        )
    })?;

    let mut file = tokio::io::BufWriter::new(file);
    let client_builder = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10));

    let client_builder = if let Some(to) = timeout {
        client_builder.timeout(std::time::Duration::from_secs(to))
    } else {
        client_builder
    };

    let client = client_builder.build().context("Building HTTP client failed")?;

    let request = client.get(source.url().as_ref())
        .build()
        .with_context(|| anyhow!("Building request for {} failed", source.url().as_ref()))?;

    let response = match client.execute(request).await {
        Ok(resp) => resp,
        Err(e) => {
            return Err(e).with_context(|| anyhow!("Downloading '{}'", source.url()))
        }
    };

    progress.lock()
        .await
        .inc_download_bytes(response.content_length().unwrap_or(0))
        .await;

    let mut stream = response.bytes_stream();
    while let Some(bytes) = stream.next().await {
        let bytes = bytes?;
        file.write_all(bytes.as_ref()).await?;
        progress.lock()
            .await
            .add_bytes(bytes.len())
            .await
    }

    progress.lock().await.finish_one_download().await;
    file.flush()
        .await
        .map_err(Error::from)
        .map(|_| ())
}


// Implementation of the 'source download' subcommand
pub async fn download(
    matches: &ArgMatches,
    config: &Configuration,
    repo: Repository,
    progressbars: ProgressBars,
) -> Result<()> {
    let force = matches.is_present("force");
    let timeout = matches.value_of("timeout")
        .map(u64::from_str)
        .transpose()
        .context("Parsing timeout argument to integer")?;
    let cache = PathBuf::from(config.source_cache_root());
    let sc = SourceCache::new(cache);
    let pname = matches
        .value_of("package_name")
        .map(String::from)
        .map(PackageName::from);
    let pvers = matches
        .value_of("package_version")
        .map(PackageVersionConstraint::try_from)
        .transpose()?;

    let matching_regexp = matches.value_of("matching")
        .map(crate::commands::util::mk_package_name_regex)
        .transpose()?;

    let progressbar = Arc::new(Mutex::new(ProgressWrapper::new(progressbars.bar())));

    let r = repo.packages()
        .filter(|p| {
            match (pname.as_ref(), pvers.as_ref(), matching_regexp.as_ref()) {
                (None, None, None)              => true,
                (Some(pname), None, None)       => p.name() == pname,
                (Some(pname), Some(vers), None) => p.name() == pname && vers.matches(p.version()),
                (None, None, Some(regex))       => regex.is_match(p.name()),

                (_, _, _) => {
                    panic!("This should not be possible, either we select packages by name and (optionally) version, or by regex.")
                },
            }
        })
        .map(|p| {
            sc.sources_for(p).into_iter().map(|source| {
                let progressbar = progressbar.clone();
                async move {
                    let source_path_exists = source.path().exists();
                    if !source_path_exists && source.download_manually() {
                        return Err(anyhow!(
                            "Cannot download source that is marked for manual download"
                        ))
                        .context(anyhow!("Creating source: {}", source.path().display()))
                        .context(anyhow!("Downloading source: {}", source.url()))
                        .map_err(Error::from);
                    }

                    if source_path_exists && !force {
                        Err(anyhow!("Source exists: {}", source.path().display()))
                    } else {
                        progressbar.lock()
                            .await
                            .inc_download_count()
                            .await;

                        if source_path_exists /* && force is implied by 'if' above*/ {
                            if let Err(e) = source.remove_file().await {
                                progressbar.lock().await.finish_one_download().await;
                                return Err(e)
                            }
                        }

                        perform_download(&source, progressbar.clone(), timeout).await?;
                        progressbar.lock().await.finish_one_download().await;
                        Ok(())
                    }
                }
            })
        })
        .flatten()
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Vec<Result<()>>>()
        .await
        .into_iter()
        .collect::<Result<()>>();

    if r.is_err() {
        progressbar.lock().await.error().await;
    } else {
        progressbar.lock().await.success().await;
    }

    debug!("r = {:?}", r);
    r
}

