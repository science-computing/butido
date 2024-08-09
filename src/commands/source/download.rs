//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::concat;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use futures::stream::{FuturesUnordered, StreamExt};
use regex::Regex;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, Semaphore};
use tracing::{info, trace, warn};

use crate::config::*;
use crate::package::Package;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;
use crate::source::*;
use crate::util::progress::ProgressBars;

const NUMBER_OF_MAX_CONCURRENT_DOWNLOADS: usize = 100;
const APP_USER_AGENT: &str = concat! {env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")};

#[derive(Clone, Debug)]
enum DownloadResult {
    Forced,
    Skipped,
    Succeeded,
    MarkedManual,
}

/// A wrapper around the indicatif::ProgressBar
///
/// A wrapper around the indicatif::ProgressBar that is used to synchronize status information from
/// the individual download jobs to the progress bar that is used to display download progress to
/// the user.
///
/// The problem this helper solves is that we only have one status bar for all downloads, and all
/// download tasks must be able to increase the number of bytes received, for example, (that is
/// displayed in the status message) but in a sync way.
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
            bar: Arc::new(Mutex::new(bar)),
        }
    }

    async fn inc_download_count(&mut self) {
        self.download_count += 1;
        self.show_progress().await;
        let bar = self.bar.lock().await;
        bar.inc_length(1);
    }

    async fn inc_download_bytes(&mut self, bytes: u64) {
        self.sum_bytes += bytes;
        self.show_progress().await;
    }

    async fn finish_one_download(&mut self) {
        self.finished_downloads += 1;
        self.bar.lock().await.inc(1);
        self.show_progress().await;
    }

    async fn add_bytes(&mut self, len: usize) {
        self.current_bytes += len;
        self.show_progress().await;
    }

    async fn show_progress(&self) {
        let bar = self.bar.lock().await;
        bar.set_message(format!("Downloading ({current_bytes}/{sum_bytes} bytes, {dlfinished}/{dlsum} downloads finished)",
                current_bytes = self.current_bytes,
                sum_bytes = self.sum_bytes,
                dlfinished = self.finished_downloads,
                dlsum = self.download_count));
    }
}

async fn perform_download(
    source: &SourceEntry,
    progress: Arc<Mutex<ProgressWrapper>>,
    timeout: Option<u64>,
) -> Result<()> {
    trace!("Downloading: {:?}", source);

    let client_builder = reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .redirect(reqwest::redirect::Policy::limited(10));

    let client_builder = if let Some(to) = timeout {
        client_builder.timeout(std::time::Duration::from_secs(to))
    } else {
        client_builder
    };

    let client = client_builder
        .build()
        .context("Building HTTP client failed")?;

    let request = client
        .get(source.url().as_ref())
        .build()
        .with_context(|| anyhow!("Building request for {} failed", source.url().as_ref()))?;

    let response = match client.execute(request).await {
        Ok(resp) => resp,
        Err(e) => return Err(e).with_context(|| anyhow!("Downloading '{}'", &source.url())),
    };

    if response.status() != reqwest::StatusCode::OK {
        return Err(anyhow!(
            "Received HTTP status code \"{}\" but \"{}\" is expected for a successful download",
            response.status(),
            reqwest::StatusCode::OK
        ))
        .with_context(|| anyhow!("Downloading \"{}\" failed", &source.url()));
    }

    progress
        .lock()
        .await
        .inc_download_bytes(response.content_length().unwrap_or(0))
        .await;

    // Check the content type to warn the user when downloading HTML files or when the server
    // didn't specify a content type.
    let content_type = &response
        .headers()
        .get("content-type")
        .map(|h| h.to_str().unwrap_or(""))
        .unwrap_or("");

    if content_type.contains("text/html") {
        warn!("The downloaded source ({}) is an HTML file", source.url());
    } else if content_type == &"" {
        warn!(
            "The server didn't specify a content type for the downloaded source ({})",
            source.url()
        );
    }
    info!(
        "The server returned content type \"{content_type}\" for \"{}\"",
        source.url()
    );

    let file = source.create().await.with_context(|| {
        anyhow!(
            "Creating source file destination: {}",
            source.path().display()
        )
    })?;
    let mut file = tokio::io::BufWriter::new(file);

    let mut stream = response.bytes_stream();
    while let Some(bytes) = stream.next().await {
        let bytes = bytes?;
        tokio::try_join!(file.write_all(bytes.as_ref()), async {
            progress.lock().await.add_bytes(bytes.len()).await;
            Ok(())
        })?;
    }

    file.flush().await.map_err(Error::from).map(|_| ())
}

async fn download_source_entry(
    source: &SourceEntry,
    download_sema: Arc<Semaphore>,
    progressbar: Arc<Mutex<ProgressWrapper>>,
    force: bool,
    timeout: Option<u64>,
) -> Result<DownloadResult> {
    let source_path_exists = source.path().exists();
    if !source_path_exists && source.download_manually() {
        return Ok(DownloadResult::MarkedManual);
    }

    if source_path_exists && !force {
        info!("Source already exists: {}", source.path().display());
        return Ok(DownloadResult::Skipped);
    }

    {
        if source_path_exists && force {
            source.remove_file().await?;
        }

        // perform the download
        progressbar.lock().await.inc_download_count().await;
        {
            let permit = download_sema.acquire_owned().await?;
            perform_download(source, progressbar.clone(), timeout).await?;
            drop(permit);
        }
        progressbar.lock().await.finish_one_download().await;

        if source_path_exists && force {
            Ok(DownloadResult::Forced)
        } else {
            Ok(DownloadResult::Succeeded)
        }
    }
}

fn find_packages(
    repo: &Repository,
    pname: Option<PackageName>,
    pvers: Option<PackageVersionConstraint>,
    matching_regexp: Option<Regex>,
) -> Result<Vec<&Package>, anyhow::Error> {
    let packages: Vec<&Package> = repo.packages()
        .filter(|p| {
            match (pname.as_ref(), pvers.as_ref(), matching_regexp.as_ref()) {
                (None, None, None) => true,
                (Some(pname), None, None) => p.name() == pname,
                (Some(pname), Some(vers), None) => p.name() == pname && vers.matches(p.version()),
                (None, None, Some(regex)) => regex.is_match(p.name()),
                (_, _, _) => {
                    panic!("This should not be possible, either we select packages by name and (optionally) version, or by regex.")
                },
            }
        })
        .collect();

    if packages.is_empty() {
        return match (pname, pvers, matching_regexp) {
            (Some(pname), None, None) => Err(anyhow!("{} not found", pname)),
            (Some(pname), Some(vers), None) => Err(anyhow!("{} {} not found", pname, vers)),
            (None, None, Some(regex)) => Err(anyhow!("{} regex not found", regex)),
            (_, _, _) => panic!("This should not be possible, either we select packages by name and (optionally) version, or by regex."),
        };
    }

    Ok(packages)
}

// Implementation of the 'source download' subcommand
pub async fn download(
    matches: &ArgMatches,
    config: &Configuration,
    repo: Repository,
    progressbars: ProgressBars,
) -> Result<()> {
    let force = matches.get_flag("force");
    let timeout = matches.get_one::<u64>("timeout").copied();
    let cache = PathBuf::from(config.source_cache_root());
    let sc = SourceCache::new(cache);
    let pname = matches
        .get_one::<String>("package_name")
        .map(|s| s.to_owned())
        .map(PackageName::from);
    let pvers = matches
        .get_one::<String>("package_version")
        .map(|s| s.to_owned())
        .map(PackageVersionConstraint::try_from)
        .transpose()?;

    let matching_regexp = matches
        .get_one::<String>("matching")
        .map(|s| crate::commands::util::mk_package_name_regex(s.as_ref()))
        .transpose()?;

    let progressbar = Arc::new(Mutex::new(ProgressWrapper::new(progressbars.bar()?)));

    let download_sema = Arc::new(tokio::sync::Semaphore::new(
        NUMBER_OF_MAX_CONCURRENT_DOWNLOADS,
    ));

    let r = find_packages(&repo, pname, pvers, matching_regexp)?;

    let r: Vec<(SourceEntry, Result<DownloadResult>)> = r
        .iter()
        .flat_map(|p| {
            sc.sources_for(p).into_iter().map(|source| {
                let download_sema = download_sema.clone();
                let progressbar = progressbar.clone();
                async move {
                    let download_result =
                        download_source_entry(&source, download_sema, progressbar, force, timeout)
                            .await;
                    (source, download_result)
                }
            })
        })
        .collect::<FuturesUnordered<_>>()
        .collect()
        .await;

    super::verify(matches, config, repo, progressbars).await?;

    Ok(())
}
