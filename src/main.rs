#[macro_use] extern crate log;

use std::path::Path;
use std::path::PathBuf;
use anyhow::Result;
use anyhow::Error;
use walkdir::WalkDir;
use indicatif::*;
use tokio::stream::StreamExt;

mod cli;
mod util;
mod package;
mod phase;
mod config;
mod repository;
use crate::config::*;
use crate::repository::Repository;
use crate::package::PackageName;
use crate::package::PackageVersion;
use crate::util::executor::DummyExecutor;
use crate::package::Tree;

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
    let repo_path    = PathBuf::from(config.repository());
    let max_packages = count_pkg_files(&repo_path);
    let progressbars = setup_progressbars(max_packages);
    let repo         = Repository::load(&repo_path, &progressbars.repo_loading)?;
    progressbars.repo_loading.finish_with_message("Repository loading finished");

    let pname = cli.value_of("package_name").map(String::from).map(PackageName::from).unwrap(); // safe by clap
    let pvers = cli.value_of("package_version").map(String::from).map(PackageVersion::from);

    let packages = if let Some(pvers) = pvers {
        repo.find(&pname, &pvers)
    } else {
        repo.find_by_name(&pname)
    };
    debug!("Found {} relevant packages", packages.len());

    let trees = tokio::stream::iter(packages.into_iter().cloned())
        .map(|p| {
            let bar = progressbars.root.add(tree_building_progress_bar(max_packages));
            bar.set_message(&format!("Building Package Tree for {}", p.name()));
            let mut tree = Tree::new();
            tree.add_package(p, &repo, &DummyExecutor::new(), &bar)?;
            Ok(tree)
        })
        .collect::<Result<Vec<_>>>()
        .await?;

    debug!("Trees loaded: {:?}", trees);

    progressbars.root.join().map_err(Error::from)
}

struct ProgressBars {
    root:           MultiProgress,
    repo_loading:   ProgressBar,
}

fn setup_progressbars(max_packages: u64) -> ProgressBars {
    let repo_loading = {
        let b = ProgressBar::new(max_packages);
        b.set_style({
            ProgressStyle::default_bar().template("[{elapsed_precise}] {pos:>3}/{len:>3} ({percent:>3}%): {bar} | {msg}")
        });

        b.set_message("Loading Repository");
        b
    };

    let root         = MultiProgress::new();
    let repo_loading = root.add(repo_loading);

    ProgressBars {
        root,
        repo_loading,
    }
}

fn tree_building_progress_bar(max: u64) -> ProgressBar {
    let b = ProgressBar::new(max);
    b.set_style({
        ProgressStyle::default_bar().template("[{elapsed_precise}] {pos:>3}/{len:>3} ({percent:>3}%): {bar} | {msg}")
    });
    b
}

fn count_pkg_files(p: &Path) -> u64 {
    WalkDir::new(p)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|d| d.file_type().is_file())
        .filter(|f| f.path().file_name().map(|name| name == "pkg.toml").unwrap_or(false))
        .count() as u64
}

