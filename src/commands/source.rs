use std::io::Write;
use std::path::PathBuf;

use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use clap::ArgMatches;
use tokio::stream::StreamExt;
use tokio::io::AsyncWriteExt;

use crate::config::*;
use crate::package::Package;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;
use crate::source::*;
use crate::util::progress::ProgressBars;

pub async fn source(matches: &ArgMatches, config: &Configuration, repo: Repository, progressbars: ProgressBars) -> Result<()> {
    match matches.subcommand() {
        Some(("verify", matches))       => verify(matches, config, repo).await,
        Some(("list-missing", matches)) => list_missing(matches, config, repo).await,
        Some(("url", matches))          => url(matches, config, repo).await,
        Some(("download", matches))     => download(matches, config, repo, progressbars).await,
        Some((other, _)) => return Err(anyhow!("Unknown subcommand: {}", other)),
        None             => return Err(anyhow!("No subcommand")),
    }
}

pub async fn verify(matches: &ArgMatches, config: &Configuration, repo: Repository) -> Result<()> {
    let source_cache_root = PathBuf::from(config.source_cache_root());
    let sc                = SourceCache::new(source_cache_root);
    let pname             = matches.value_of("package_name").map(String::from).map(PackageName::from);
    let pvers             = matches.value_of("package_version").map(String::from).map(PackageVersionConstraint::new).transpose()?;

    let packages = repo.packages()
        .filter(|p| pname.as_ref().map(|n| p.name() == n).unwrap_or(true))
        .filter(|p| pvers.as_ref().map(|v| v.matches(p.version())).unwrap_or(true));

    let mut out = std::io::stdout();
    verify_impl(packages, &sc, &mut out).await
}

pub (in crate::commands) async fn verify_impl<'a, I>(packages: I, sc: &SourceCache, output: &mut Write) -> Result<()>
    where I: Iterator<Item = &'a Package> + 'a
{
    if let Err(_) = packages
        .map(|p| sc.sources_for(p).into_iter())
        .flatten()
        .map(|source| async move {
            if source.path().exists() {
                match source.verify_hash().await {
                    Ok(true)  => Ok(format!("Ok: {}", source.path().display())),
                    Ok(false) => Err(format!("Hash Mismatch: {}", source.path().display())),
                    Err(e)    => Err(format!("Hash verification failed: {}", e.to_string())), // TODO: make me nice
                }
            } else {
                Err(format!("Source missing: {}", source.path().display()))
            }
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Vec<std::result::Result<String, String>>>()
        .await
        .into_iter()
        .inspect(|r| { let _ = writeln!(output, "{}", match r { Ok(s) | Err(s) => s }); })
        .map(|r| r.map(|_| ()).map_err(|_| ()))
        .collect::<std::result::Result<(), ()>>()
    {
        Err(anyhow!("At least one package failed with source verification"))
    } else {
        Ok(())
    }
}

pub async fn list_missing(_: &ArgMatches, config: &Configuration, repo: Repository) -> Result<()> {
    let sc          = SourceCache::new(PathBuf::from(config.source_cache_root()));
    let out         = std::io::stdout();
    let mut outlock = out.lock();

    repo.packages()
        .map(|p| {
            for source in sc.sources_for(p) {
                if !source.exists() {
                    writeln!(outlock, "{} {} -> {}", p.name(), p.version(), source.path().display())?;
                }
            }

            Ok(())
        })
        .collect()
}

pub async fn url(matches: &ArgMatches, config: &Configuration, repo: Repository) -> Result<()> {
    let out         = std::io::stdout();
    let mut outlock = out.lock();

    let pname = matches.value_of("package_name").map(String::from).map(PackageName::from);
    let pvers = matches.value_of("package_version").map(String::from).map(PackageVersionConstraint::new).transpose()?;

    repo.packages()
        .filter(|p| pname.as_ref().map(|n| p.name() == n).unwrap_or(true))
        .filter(|p| pvers.as_ref().map(|v| v.matches(p.version())).unwrap_or(true))
        .map(|p| {
            p.sources()
                .iter()
                .map(|source| writeln!(outlock, "{} {} -> {}", p.name(), p.version(), source.url()).map_err(Error::from))
                .collect()
        })
        .collect()
}

pub async fn download(matches: &ArgMatches, config: &Configuration, repo: Repository, progressbars: ProgressBars) -> Result<()> {
    let force = matches.is_present("force");
    let cache = PathBuf::from(config.source_cache_root());
    let sc    = SourceCache::new(cache);
    let pname = matches.value_of("package_name").map(String::from).map(PackageName::from);
    let pvers = matches.value_of("package_version").map(String::from).map(PackageVersionConstraint::new).transpose()?;
    let multi = indicatif::MultiProgress::new();

    let r = repo.packages()
        .filter(|p| pname.as_ref().map(|n| p.name() == n).unwrap_or(true))
        .filter(|p| pvers.as_ref().map(|v| v.matches(p.version())).unwrap_or(true))
        .map(|p| {
            sc.sources_for(p)
                .into_iter()
                .map(|source| {
                    let bar = multi.add(progressbars.download_bar(source.url()));
                    async move {
                        if source.exists() && !force {
                            Err(anyhow!("Source exists: {}", source.path().display()))
                        } else {
                            if source.exists() {
                                let _ = source.remove_file().await?;
                            }

                            trace!("Starting download...");
                            let file = source.create().await?;
                            let mut file = tokio::io::BufWriter::new(file);
                            let response = reqwest::get(source.url().as_ref()).await?;
                            if let Some(len) = response.content_length() {
                                bar.set_length(len);
                            }
                            let mut stream = reqwest::get(source.url().as_ref()).await?.bytes_stream();
                            while let Some(bytes) = stream.next().await {
                                let bytes = bytes?;
                                file.write_all(bytes.as_ref()).await?;
                                bar.inc(bytes.len() as u64);
                            }

                            file.flush().await?;
                            bar.finish_with_message("Finished download");
                            Ok(())
                        }
                    }
                })
        })
        .flatten()
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<()>>()
        .await;
    multi.join()?;
    r
}

