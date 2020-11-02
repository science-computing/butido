#[macro_use] extern crate log as logcrate;
use logcrate::debug;

use std::path::Path;
use std::path::PathBuf;
use std::collections::BTreeMap;

use anyhow::anyhow;
use anyhow::Result;
use anyhow::Error;
use walkdir::WalkDir;
use indicatif::*;
use tokio::stream::StreamExt;
use clap_v3::ArgMatches;

mod cli;
mod job;
mod util;
mod log;
mod package;
mod phase;
mod config;
mod repository;
mod filestore;
mod ui;
use crate::config::*;
use crate::repository::Repository;
use crate::package::PackageName;
use crate::package::PackageVersion;
use crate::package::Tree;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::util::progress::ProgressBars;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = env_logger::try_init()?;
    debug!("Debugging enabled");

    let cli = cli::cli();
    let cli = cli.get_matches();

    let mut config = ::config::Config::default();
    config
        .merge(::config::File::with_name("config"))?
        .merge(::config::Environment::with_prefix("YABOS"))?;
        // Add in settings from the environment (with a prefix of YABOS)
        // Eg.. `YABOS_DEBUG=1 ./target/app` would set the `debug` key
    //

    let config: Configuration = config.try_into::<NotValidatedConfiguration>()?.validate()?;
    let repo_path             = PathBuf::from(config.repository());
    let max_packages          = count_pkg_files(&repo_path, ProgressBar::new_spinner());
    let mut progressbars      = ProgressBars::setup();

    let repo = {
        let bar = progressbars.repo_loading();
        bar.set_length(max_packages);
        let repo = Repository::load(&repo_path, &bar)?;
        bar.finish_with_message("Repository loading finished");
        repo
    };

    match cli.subcommand() {
        ("build", Some(matches))        => {
            let bar_tree_building = progressbars.tree_building();
            bar_tree_building.set_length(max_packages);

            let bar_release_loading = progressbars.release_loading();
            bar_release_loading.set_length(max_packages);

            let bar_staging_loading = progressbars.staging_loading();
            bar_staging_loading.set_length(max_packages);

            build(matches, &config, repo, bar_tree_building, bar_release_loading, bar_staging_loading).await?
        },
        ("what-depends", Some(matches)) => {
            let bar = progressbars.what_depends();
            bar.set_length(max_packages);
            what_depends(matches, repo, bar).await?
        },

        (other, _) => return Err(anyhow!("Unknown subcommand: {}", other)),
    }

    progressbars.into_inner().join().map_err(Error::from)
}

async fn build<'a>(matches: &ArgMatches,
               config: &Configuration<'a>,
               repo: Repository,
               bar_tree_building: ProgressBar,
               bar_release_loading: ProgressBar,
               bar_staging_loading: ProgressBar)
    -> Result<()>
{
    let release_dir  = async move {
        let variables = BTreeMap::new();
        let p = config.releases_directory(&variables)?;
        debug!("Loading release directory: {}", p.display());
        let r = ReleaseStore::load(&p, bar_release_loading.clone());
        if r.is_ok() {
            bar_release_loading.finish_with_message("Loaded releases successfully");
        } else {
            bar_release_loading.finish_with_message("Failed to load releases");
        }
        r
    };

    let staging_dir = async move {
        let variables = BTreeMap::new();
        let p = config.staging_directory(&variables)?;
        debug!("Loading staging directory: {}", p.display());
        let r = StagingStore::load(&p, bar_staging_loading.clone());
        if r.is_ok() {
            bar_staging_loading.finish_with_message("Loaded staging successfully");
        } else {
            bar_staging_loading.finish_with_message("Failed to load staging");
        }
        r
    };


    let pname = matches.value_of("package_name")
        .map(String::from)
        .map(PackageName::from)
        .unwrap(); // safe by clap

    let pvers = matches.value_of("package_version")
        .map(String::from)
        .map(PackageVersion::from);

    let packages = if let Some(pvers) = pvers {
        repo.find(&pname, &pvers)
    } else {
        repo.find_by_name(&pname)
    };
    debug!("Found {} relevant packages", packages.len());

    let trees = tokio::stream::iter(packages.into_iter().cloned())
        .map(|p| {
            let mut tree = Tree::new();
            tree.add_package(p, &repo, bar_tree_building.clone())?;
            Ok(tree)
        })
        .collect::<Result<Vec<_>>>()
        .await?;

    bar_tree_building.finish_with_message("Finished loading Tree");

    debug!("Trees loaded: {:?}", trees);
    let mut out = std::io::stderr();
    for tree in trees {
        tree.debug_print(&mut out)?;
    }

    Ok(())
}

async fn what_depends(matches: &ArgMatches, repo: Repository, progress: ProgressBar) -> Result<()> {
    use filters::filter::Filter;

    let print_runtime_deps     = getbool(matches, "dependency_type", crate::cli::IDENT_DEPENDENCY_TYPE_RUNTIME);
    let print_build_deps       = getbool(matches, "dependency_type", crate::cli::IDENT_DEPENDENCY_TYPE_BUILD);
    let print_sys_deps         = getbool(matches, "dependency_type", crate::cli::IDENT_DEPENDENCY_TYPE_SYSTEM);
    let print_sys_runtime_deps = getbool(matches, "dependency_type", crate::cli::IDENT_DEPENDENCY_TYPE_SYSTEM_RUNTIME);

    let package_filter = {
        let name = matches.value_of("package_name").map(String::from).unwrap();

        crate::util::filters::build_package_filter_by_dependency_name(
            name,
            print_sys_deps,
            print_sys_runtime_deps,
            print_build_deps,
            print_runtime_deps
        )
    };

    let format = matches.value_of("list-format").unwrap(); // safe by clap default value
    let mut stdout = std::io::stdout();
    let iter = repo.packages().filter(|package| package_filter.filter(package));
    ui::print_packages(&mut stdout,
                       format,
                       iter,
                       print_runtime_deps,
                       print_build_deps,
                       print_sys_deps,
                       print_sys_runtime_deps)
}

fn count_pkg_files(p: &Path, progress: ProgressBar) -> u64 {
    WalkDir::new(p)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|d| d.file_type().is_file())
        .filter(|f| f.path().file_name().map(|name| name == "pkg.toml").unwrap_or(false))
        .inspect(|_| progress.tick())
        .count() as u64
}

fn getbool(m: &ArgMatches, name: &str, cmp: &str) -> bool {
    // unwrap is safe here because clap is configured with default values
    m.values_of(name).unwrap().any(|v| v == cmp)
}

