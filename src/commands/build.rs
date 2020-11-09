use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use clap_v3::ArgMatches;
use diesel::PgConnection;
use logcrate::debug;

use crate::config::*;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::job::JobSet;
use crate::orchestrator::OrchestratorSetup;
use crate::package::PackageName;
use crate::package::PackageVersion;
use crate::package::Tree;
use crate::repository::Repository;
use crate::util::docker::ImageName;
use crate::util::progress::ProgressBars;

pub async fn build<'a>(matches: &ArgMatches,
               progressbars: ProgressBars,
               database_connection: PgConnection,
               config: &Configuration<'a>,
               repo: Repository,
               repo_path: &Path,
               max_packages: u64)
    -> Result<()>
{
    use crate::db::models::{
        Package,
        GitHash,
        Image,
        Submit,
    };

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

    // We only support building one package per call.
    // Everything else is invalid
    if packages.len() > 1 {
        return Err(anyhow!("Found multiple packages ({}). Cannot decide which one to build", packages.len()))
    }
    let package = *packages.get(0).ok_or_else(|| anyhow!("Found no package."))?;

    let release_dir  = {
        let bar_release_loading = progressbars.release_loading();
        bar_release_loading.set_length(max_packages);

        let variables = BTreeMap::new();
        let p = config.releases_directory(&variables)?;
        debug!("Loading release directory: {}", p.display());
        let r = ReleaseStore::load(&p, bar_release_loading.clone());
        if r.is_ok() {
            bar_release_loading.finish_with_message("Loaded releases successfully");
        } else {
            bar_release_loading.finish_with_message("Failed to load releases");
        }
        r.map(RwLock::new).map(Arc::new)?
    };

    let staging_dir = {
        let bar_staging_loading = progressbars.staging_loading();
        bar_staging_loading.set_length(max_packages);

        let variables = BTreeMap::new();
        let p = config.staging_directory(&variables)?;
        debug!("Loading staging directory: {}", p.display());
        let r = StagingStore::load(&p, bar_staging_loading.clone());
        if r.is_ok() {
            bar_staging_loading.finish_with_message("Loaded staging successfully");
        } else {
            bar_staging_loading.finish_with_message("Failed to load staging");
        }
        r.map(RwLock::new).map(Arc::new)?
    };

    let tree = {
        let bar_tree_building = progressbars.tree_building();
        bar_tree_building.set_length(max_packages);

        let mut tree = Tree::new();
        tree.add_package(package.clone(), &repo, bar_tree_building.clone())?;

        bar_tree_building.finish_with_message("Finished loading Tree");
        tree
    };

    trace!("Setting up database jobs for Package, GitHash, Image");
    let db_package = async { Package::create_or_fetch(&database_connection, &package) };
    let db_githash = async { GitHash::create_or_fetch(&database_connection, &hash_str) };
    let db_image   = async { Image::create_or_fetch(&database_connection, &image_name) };

    trace!("Running database jobs for Package, GitHash, Image");
    let (db_package, db_githash, db_image) = tokio::join!(
        db_package,
        db_githash,
        db_image
    );

    let (db_package, db_githash, db_image) = (db_package?, db_githash?, db_image?);

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
        .progress_generator(progressbars)
        .endpoint_config(endpoint_configurations)
        .staging_store(staging_dir)
        .release_store(release_dir)
        .database(database_connection)
        .submit(submit)
        .file_log_sink_factory(None)
        .jobsets(jobsets)
        .build()
        .setup()
        .await?;

    info!("Running orchestrator...");
    let res         = orch.run().await?;
    let out         = std::io::stdout();
    let mut outlock = out.lock();

    writeln!(outlock, "Packages created:")?;
    res.into_iter()
        .map(|path| writeln!(outlock, "-> {}", path.display()).map_err(Error::from))
        .collect::<Result<_>>()
}
