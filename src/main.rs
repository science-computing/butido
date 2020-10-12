#[macro_use] extern crate log;

use std::path::Path;
use std::path::PathBuf;
use anyhow::Result;
use anyhow::Error;
use walkdir::WalkDir;

mod cli;
mod util;
mod package;
mod phase;
mod config;
mod repository;
use crate::config::DockerConfig;
use crate::repository::Repository;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = cli::cli();
    let cli = cli.get_matches();

    let mut config = ::config::Config::default();
    config
        .merge(::config::File::with_name("config"))?
        .merge(::config::Environment::with_prefix("YABOS"))?;
        // Add in settings from the environment (with a prefix of YABOS)
        // Eg.. `YABOS_DEBUG=1 ./target/app` would set the `debug` key
    //

    let docker_config = config.get::<DockerConfig>("docker")?;
    let repo_path    = PathBuf::from(config.get_str("repository")?);

    let progressbars = setup_progressbars(&repo_path);
    let repo         = Repository::load(&repo_path, &progressbars.repo_loading)?;
    progressbars.repo_loading.finish_with_message("Repository loading finished");

    progressbars.root.join().map_err(Error::from)
}

struct ProgressBars {
    root:           indicatif::MultiProgress,
    repo_loading:   indicatif::ProgressBar,
}

fn setup_progressbars(root_pkg_path: &Path) -> ProgressBars {
    use indicatif::*;

    let repo_loading = {
        let b = ProgressBar::new(count_pkg_files(root_pkg_path));
        b.set_style({
            ProgressStyle::default_bar().template("[{elapsed_precise}] {pos:>3}/{len:>3} ({percent}%): {bar} | {msg}")
        });
        b
    };

    let root         = MultiProgress::new();
    let repo_loading = root.add(repo_loading);

    ProgressBars {
        root,
        repo_loading,
    }
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

