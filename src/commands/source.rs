use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use anyhow::anyhow;
use clap_v3::ArgMatches;

use crate::config::*;
use crate::package::PackageName;
use crate::repository::Repository;
use crate::source::*;

pub async fn source<'a>(matches: &ArgMatches, config: &Configuration<'a>, repo: Repository) -> Result<()> {
    match matches.subcommand() {
        ("verify", Some(matches))     => verify(matches, config, repo).await,
        (other, _) => return Err(anyhow!("Unknown subcommand: {}", other)),
    }
}

pub async fn verify<'a>(matches: &ArgMatches, config: &Configuration<'a>, repo: Repository) -> Result<()> {
    use tokio::stream::StreamExt;

    let source_cache_root = PathBuf::from(config.source_cache_root());
    let sc                = SourceCache::new(source_cache_root);
    let pname             = matches.value_of("package_name").map(String::from).map(PackageName::from);

    repo.packages()
        .filter(|p| pname.as_ref().map(|n| p.name() == n).unwrap_or(true))
        .map(|p| {
            let source = sc.source_for(p);
            async move {
                let out = std::io::stdout();
                if source.exists() {
                    if source.verify_hash().await? {
                        writeln!(out.lock(), "Ok: {}", source.path().display())?;
                    } else {
                        writeln!(out.lock(), "Hash Mismatch: {}", source.path().display())?;
                    }
                } else {
                    writeln!(out.lock(), "Source missing: {}", source.path().display())?;
                }

                Ok(())
            }
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Result<()>>()
        .await
}
