//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::collections::HashSet;
use std::concat;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Context, Error, Result};
use ascii_table::{Align, AsciiTable};
use clap::ArgMatches;
use futures::stream::{FuturesOrdered, StreamExt};
use regex::Regex;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, Semaphore};
use tracing::{debug, error, info, trace, warn};

use crate::config::Configuration;
use crate::package::condition::ConditionData;
use crate::package::{Dag, Package, PackageName, PackageVersionConstraint};
use crate::repository::Repository;
use crate::source::{SourceCache, SourceEntry};
use crate::util::docker::ImageNameLookup;
use crate::util::progress::ProgressBars;
use crate::util::EnvironmentVariableName;

const NUMBER_OF_MAX_CONCURRENT_DOWNLOADS: usize = 100;
const APP_USER_AGENT: &str = concat! {env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")};

#[derive(Clone, Debug)]
enum DownloadResult {
    Forced,
    Skipped,
    Succeeded,
    MarkedManual,
}

impl fmt::Display for DownloadResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DownloadResult::Forced => write!(f, "forced"),
            DownloadResult::Skipped => write!(f, "skipped"),
            DownloadResult::Succeeded => write!(f, "succeeded"),
            DownloadResult::MarkedManual => write!(f, "marked manual"),
        }
    }
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
        let current_mbytes = (self.current_bytes as f64) / 1_000_000_f64;
        let sum_mbytes = (self.sum_bytes as f64) / 1_000_000_f64;
        bar.set_message(format!(
            "Downloading ({:.2}/{:.2} MB, {dlfinished}/{dlsum} downloads finished)",
            current_mbytes,
            sum_mbytes,
            dlfinished = self.finished_downloads,
            dlsum = self.download_count
        ));
    }
}

struct DownloadJob {
    package: Package,
    source_entry: SourceEntry,
    download_result: Result<DownloadResult>,
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
    let recursive = matches.get_flag("recursive");
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

    let found_packages = find_packages(&repo, pname, pvers, matching_regexp)?;

    let packages_to_download: HashSet<Package> = match recursive {
        true => {
            debug!("Finding package dependencies recursively");

            let image_name_lookup = ImageNameLookup::create(config.docker().images())?;
            let image_name = matches
                .get_one::<String>("image")
                .map(|s| image_name_lookup.expand(s))
                .transpose()?;

            let additional_env = matches
                .get_many::<String>("env")
                .unwrap_or_default()
                .map(AsRef::as_ref)
                .map(crate::util::env::parse_to_env)
                .collect::<Result<Vec<(EnvironmentVariableName, String)>>>()?;

            let condition_data = ConditionData {
                image_name: image_name.as_ref(),
                env: &additional_env,
            };

            let dependencies: Vec<Package> = found_packages
                .iter()
                .flat_map(|package| {
                    Dag::for_root_package((*package).clone(), &repo, None, &condition_data)
                        .map(|d| d.dag().graph().node_weights().cloned().collect::<Vec<_>>())
                        .unwrap()
                })
                .collect();

            HashSet::from_iter(dependencies)
        }
        false => HashSet::from_iter(found_packages.into_iter().cloned()),
    };

    let download_results: Vec<(SourceEntry, Result<DownloadResult>)> = packages_to_download
        .iter()
        .flat_map(|p| {
            //download the sources and wait for all packages to finish
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
        .collect::<FuturesOrdered<_>>()
        .collect()
        .await;

    let mut r: Vec<DownloadJob> = download_results
        .into_iter()
        .zip(packages_to_download)
        .map(|r| {
            let download_result = r.0;
            let package = r.1;
            DownloadJob {
                package,
                source_entry: download_result.0,
                download_result: download_result.1,
            }
        })
        .collect();

    {
        let mut ascii_table = AsciiTable::default();
        ascii_table.set_max_width(
            terminal_size::terminal_size()
                .map(|tpl| tpl.0 .0 as usize)
                .unwrap_or(80),
        );
        ascii_table.column(0).set_header("#").set_align(Align::Left);
        ascii_table
            .column(1)
            .set_header("Package name")
            .set_align(Align::Left);
        ascii_table
            .column(2)
            .set_header("Version")
            .set_align(Align::Left);
        ascii_table
            .column(3)
            .set_header("Source name")
            .set_align(Align::Left);
        ascii_table
            .column(4)
            .set_header("Status")
            .set_align(Align::Left);
        ascii_table
            .column(5)
            .set_header("Path")
            .set_align(Align::Left);

        let numbers: Vec<usize> = (0..r.len()).map(|n| n + 1).collect();
        r.sort_by(|a, b| {
            a.source_entry
                .package_name()
                .partial_cmp(b.source_entry.package_name())
                .unwrap()
        });
        let source_paths: Vec<String> = r.iter().map(|v| v.source_entry.path_as_string()).collect();

        let data: Vec<Vec<&dyn fmt::Display>> = r
            .iter()
            .enumerate()
            .map(|(i, v)| {
                debug!("source_entry: {:#?}", v.source_entry);
                let n = &numbers[i];
                let mut row: Vec<&dyn fmt::Display> = vec![
                    n,
                    v.source_entry.package_name(),
                    v.source_entry.package_version(),
                    v.source_entry.package_source_name(),
                ];
                if v.download_result.is_ok() {
                    let result = v.download_result.as_ref().unwrap() as &dyn fmt::Display;
                    row.push(result);
                } else {
                    row.push(&"failed");
                }
                row.push(&source_paths[i]);
                row
            })
            .collect();

        ascii_table.print(data);
    }

    for p in &r {
        if p.download_result.is_err() {
            error!("{}: {:?}", p.source_entry.package_name(), p.download_result);
        }
    }

    let packages_to_verify = r.iter().filter_map(|j| {
        if let Ok(r) = &j.download_result {
            match r {
                DownloadResult::MarkedManual => None,
                _ => Some(&j.package),
            }
        } else {
            None
        }
    });

    let sc = SourceCache::new(config.source_cache_root().clone());
    super::verify_impl(packages_to_verify, &sc, &progressbars).await?;

    Ok(())
}
