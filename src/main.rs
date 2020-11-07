#[macro_use] extern crate log as logcrate;
#[macro_use] extern crate diesel;
use logcrate::debug;

use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::collections::BTreeMap;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use anyhow::Error;
use walkdir::WalkDir;
use indicatif::*;
use tokio::stream::StreamExt;
use clap_v3::ArgMatches;
use diesel::PgConnection;
use diesel::prelude::*;

mod cli;
mod job;
mod endpoint;
mod util;
mod log;
mod package;
mod phase;
mod config;
mod repository;
mod filestore;
mod ui;
mod orchestrator;
mod schema;
mod db;
use crate::config::*;
use crate::repository::Repository;
use crate::package::PackageName;
use crate::package::PackageVersion;
use crate::package::Tree;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::util::progress::ProgressBars;
use crate::orchestrator::Orchestrator;
use crate::orchestrator::OrchestratorSetup;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = env_logger::try_init()?;
    debug!("Debugging enabled");

    let cli = cli::cli();
    let cli = cli.get_matches();

    let mut config = ::config::Config::default();
    config
        .merge(::config::File::with_name("config"))?
        .merge(::config::Environment::with_prefix("BUTIDO"))?;
        // Add in settings from the environment (with a prefix of YABOS)
        // Eg.. `YABOS_DEBUG=1 ./target/app` would set the `debug` key
    //

    let config: Configuration = config.try_into::<NotValidatedConfiguration>()?.validate()?;
    let repo_path             = PathBuf::from(config.repository());
    let _                     = crate::ui::package_repo_cleanness_check(&repo_path)?;
    let max_packages          = count_pkg_files(&repo_path, ProgressBar::new_spinner());
    let mut progressbars      = ProgressBars::setup();

    let mut load_repo = || -> Result<Repository> {
        let bar = progressbars.repo_loading();
        bar.set_length(max_packages);
        let repo = Repository::load(&repo_path, &bar)?;
        bar.finish_with_message("Repository loading finished");
        Ok(repo)
    };

    let db_connection_config = crate::db::parse_db_connection_config(&config, &cli);
    match cli.subcommand() {
        ("db", Some(matches))           => db::interface(db_connection_config, matches, &config)?,
        ("build", Some(matches))        => {
            let conn = crate::db::establish_connection(db_connection_config)?;

            let repo = load_repo()?;
            let bar_tree_building = progressbars.tree_building();
            bar_tree_building.set_length(max_packages);

            let bar_release_loading = progressbars.release_loading();
            bar_release_loading.set_length(max_packages);

            let bar_staging_loading = progressbars.staging_loading();
            bar_staging_loading.set_length(max_packages);

            build(matches, conn, &config, repo, &repo_path, bar_tree_building, bar_release_loading, bar_staging_loading).await?
        },
        ("what-depends", Some(matches)) => {
            let repo = load_repo()?;
            let bar = progressbars.what_depends();
            bar.set_length(max_packages);
            what_depends(matches, repo, bar).await?
        },

        ("dependencies-of", Some(matches)) => {
            let repo = load_repo()?;
            let bar = progressbars.what_depends();
            bar.set_length(max_packages);
            dependencies_of(matches, repo, bar).await?
        },

        (other, _) => return Err(anyhow!("Unknown subcommand: {}", other)),
    }

    progressbars.into_inner().join().map_err(Error::from)
}

async fn build<'a>(matches: &ArgMatches,
               database_connection: PgConnection,
               config: &Configuration<'a>,
               repo: Repository,
               repo_path: &Path,
               bar_tree_building: ProgressBar,
               bar_release_loading: ProgressBar,
               bar_staging_loading: ProgressBar)
    -> Result<()>
{
    use crate::db::models::{
        Package,
        GitHash,
        Image,
        Submit,
    };
    use schema::packages;
    use schema::githashes;
    use schema::images;
    use crate::job::JobSet;
    use std::sync::Arc;
    use std::sync::RwLock;
    use crate::util::docker::ImageName;

    let now = chrono::offset::Local::now().naive_local();
    let submit_id = uuid::Uuid::new_v4();
    info!("Submit {}, started {}", submit_id, now);

    let image_name = matches.value_of("image").map(String::from).map(ImageName::from).unwrap(); // safe by clap
    if config.docker().verify_images_present() {
        if !config.docker().images().iter().any(|img| image_name == *img) {
            return Err(anyhow!("Requested build image {} is not in the configured images"))
                .with_context(|| anyhow!("Available images: {:?}", config.docker().images()))
                .with_context(|| anyhow!("Image present verification failed"))
                .map_err(Error::from)
        }
    }

    debug!("Getting repository HEAD");
    let hash_str   = crate::util::git::get_repo_head_commit_hash(repo_path)?;
    trace!("Repository HEAD = {}", hash_str);
    let phases = config.available_phases();

    let endpoint_configurations = config.docker().endpoints()
        .iter()
        .cloned()
        .map(|ep_cfg| {
            crate::endpoint::EndpointConfiguration::builder()
                .endpoint(ep_cfg)
                .required_images(config.docker().images().clone())
                .required_docker_versions(config.docker().docker_versions().clone())
                .required_docker_api_versions(config.docker().docker_api_versions().clone())
                .build()
        })
        .collect();
    info!("Endpoint config build");

    let pname = matches.value_of("package_name")
        .map(String::from)
        .map(PackageName::from)
        .unwrap(); // safe by clap

    let pvers = matches.value_of("package_version")
        .map(String::from)
        .map(PackageVersion::from);
    info!("We want {} ({:?})", pname, pvers);

    let packages = if let Some(pvers) = pvers {
        repo.find(&pname, &pvers)
    } else {
        repo.find_by_name(&pname)
    };
    debug!("Found {} relevant packages", packages.len());

    /// We only support building one package per call.
    /// Everything else is invalid
    if packages.len() > 1 {
        return Err(anyhow!("Found multiple packages ({}). Cannot decide which one to build", packages.len()))
    }
    let package = *packages.get(0).ok_or_else(|| anyhow!("Found no package."))?;

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
        r.map(RwLock::new).map(Arc::new)
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
        r.map(RwLock::new).map(Arc::new)
    };

    let tree = async {
        let mut tree = Tree::new();
        tree.add_package(package.clone(), &repo, bar_tree_building.clone())?;
        bar_tree_building.finish_with_message("Finished loading Tree");
        Ok(tree) as Result<Tree>
    };

    trace!("Setting up database jobs for Package, GitHash, Image");
    let db_package = async { Package::create_or_fetch(&database_connection, &package) };
    let db_githash = async { GitHash::create_or_fetch(&database_connection, &hash_str) };
    let db_image   = async { Image::create_or_fetch(&database_connection, &image_name) };

    trace!("Running database jobs for Package, GitHash, Image");
    let (tree, db_package, db_githash, db_image) = tokio::join!(
        tree,
        db_package,
        db_githash,
        db_image
    );

    let (tree, db_package, db_githash, db_image) =
        (tree?, db_package?, db_githash?, db_image?);

    trace!("Database jobs for Package, GitHash, Image finished successfully");
    trace!("Creating Submit in database");
    let submit = Submit::create(&database_connection,
        &tree,
        &now,
        &submit_id,
        &db_image,
        &db_package,
        &db_githash)?;
    trace!("Creating Submit in database finished successfully");

    trace!("Setting up job sets");
    let jobsets = JobSet::sets_from_tree(tree, image_name, phases.clone())?;
    trace!("Setting up job sets finished successfully");

    trace!("Setting up Orchestrator");
    let orch = OrchestratorSetup::builder()
        .endpoint_config(endpoint_configurations)
        .staging_store(staging_dir.await?)
        .release_store(release_dir.await?)
        .database(database_connection)
        .submit(submit)
        .file_log_sink_factory(None)
        .jobsets(jobsets)
        .build()
        .setup()
        .await?;

    info!("Running orchestrator...");
    orch.run().await
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

async fn dependencies_of(matches: &ArgMatches, repo: Repository, progress: ProgressBar) -> Result<()> {
    use filters::filter::Filter;

    let package_filter = {
        let name = matches.value_of("package_name").map(String::from).map(PackageName::from).unwrap();
        trace!("Checking for package with name = {}", name);

        crate::util::filters::build_package_filter_by_name(name)
    };

    let format = matches.value_of("list-format").unwrap(); // safe by clap default value
    let mut stdout = std::io::stdout();
    let iter = repo.packages().filter(|package| package_filter.filter(package))
        .inspect(|pkg| trace!("Found package: {:?}", pkg));

    let print_runtime_deps     = getbool(matches, "dependency_type", crate::cli::IDENT_DEPENDENCY_TYPE_RUNTIME);
    let print_build_deps       = getbool(matches, "dependency_type", crate::cli::IDENT_DEPENDENCY_TYPE_BUILD);
    let print_sys_deps         = getbool(matches, "dependency_type", crate::cli::IDENT_DEPENDENCY_TYPE_SYSTEM);
    let print_sys_runtime_deps = getbool(matches, "dependency_type", crate::cli::IDENT_DEPENDENCY_TYPE_SYSTEM_RUNTIME);

    trace!("Printing packages with format = '{}', runtime: {}, build: {}, sys: {}, sys_rt: {}",
           format,
           print_runtime_deps,
           print_build_deps,
           print_sys_deps,
           print_sys_runtime_deps);

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

