//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Implementation of the 'build' subcommand

use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use colored::Colorize;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use diesel::ExpressionMethods;
use diesel::PgConnection;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use itertools::Itertools;
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use tracing::Instrument;
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

use crate::config::*;
use crate::db::models::{EnvVar, GitHash, Image, Job, Package, Submit};
use crate::filestore::path::StoreRoot;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::job::JobResource;
use crate::log::LogItem;
use crate::orchestrator::OrchestratorSetup;
use crate::package::condition::ConditionData;
use crate::package::Dag;
use crate::package::PackageName;
use crate::package::PackageVersion;
use crate::package::Shebang;
use crate::repository::Repository;
use crate::schema;
use crate::source::SourceCache;
use crate::util::docker::ImageNameLookup;
use crate::util::progress::ProgressBars;
use crate::util::EnvironmentVariableName;

/// Implementation of the "build" subcommand
#[allow(clippy::too_many_arguments)]
pub async fn build(
    repo_root: &Path,
    matches: &ArgMatches,
    progressbars: ProgressBars,
    database_pool: Pool<ConnectionManager<PgConnection>>,
    config: &Configuration,
    repo: Repository,
    repo_path: &Path,
) -> Result<()> {
    let command_span = tracing::debug_span!("command-build");

    let loading_span = tracing::debug_span!(parent: &command_span, "loading");
    // There's no async code for a long time in this function, so this is safe.
    let loading_span_guard = loading_span.enter();

    let git_repo = git2::Repository::open(repo_path)
        .with_context(|| anyhow!("Opening repository at {}", repo_path.display()))?;

    let now = chrono::offset::Local::now().naive_local();

    let shebang = Shebang::from({
        matches
            .get_one::<String>("shebang")
            .map(|s| s.to_owned())
            .unwrap_or_else(|| config.shebang().clone())
    });

    let image_name_lookup = ImageNameLookup::create(config.docker().images())?;
    let image_name = matches
        .get_one::<String>("image")
        .map(|s| image_name_lookup.expand(s))
        .unwrap()?; // safe by clap

    debug!("Getting repository HEAD");
    let hash_str = crate::util::git::get_repo_head_commit_hash(&git_repo)?;
    trace!("Repository HEAD = {}", hash_str);
    let phases = config.available_phases();

    let mut endpoint_configurations = config
        .docker()
        .endpoints()
        .iter()
        .map(|(ep_name, ep_cfg)| {
            crate::endpoint::EndpointConfiguration::builder()
                .endpoint_name(ep_name.clone())
                .endpoint(ep_cfg.clone())
                .required_images(
                    config
                        .docker()
                        .images()
                        .iter()
                        .map(|img| img.name.clone())
                        .collect::<Vec<_>>(),
                )
                .required_docker_versions(config.docker().docker_versions().clone())
                .required_docker_api_versions(config.docker().docker_api_versions().clone())
                .build()
        })
        .collect::<Vec<_>>();
    {
        // Because we're loading always sequentially, to have a bit more spread over the endpoints,
        // shuffle the endpoints here. Not a perfect solution, but a working one.
        use rand::seq::SliceRandom;
        let mut rng = rand::rng();
        endpoint_configurations.shuffle(&mut rng);
    }
    info!("Endpoint config build");

    let pname = matches
        .get_one::<String>("package_name")
        .map(|s| s.to_owned())
        .map(PackageName::from)
        .unwrap(); // safe by clap

    let pvers = matches
        .get_one::<String>("package_version")
        .map(|s| s.to_owned())
        .map(PackageVersion::from);
    info!("We want {} ({:?})", pname, pvers);

    let additional_env = matches
        .get_many::<String>("env")
        .unwrap_or_default()
        .map(|s| crate::util::env::parse_to_env(s.as_ref()))
        .collect::<Result<Vec<(EnvironmentVariableName, String)>>>()?;

    let packages = if let Some(pvers) = pvers {
        debug!(
            "Searching for package with version: '{}' '{}'",
            pname, pvers
        );
        repo.find(&pname, &pvers)
    } else {
        debug!("Searching for package by name: '{}'", pname);
        repo.find_by_name(&pname)
    };
    debug!("Found {} relevant packages", packages.len());

    // We only support building one package per call.
    // Everything else is invalid
    if packages.len() > 1 {
        return Err(anyhow!(
            "Found multiple packages ({}). Cannot decide which one to build",
            packages.len()
        ));
    }
    let package = *packages
        .first()
        .ok_or_else(|| anyhow!("Found no package."))?;

    let release_stores = config
        .release_stores()
        .iter()
        .map(|storename| {
            let bar_release_loading = progressbars.bar()?;

            let p = config.releases_directory().join(storename);
            let p_str = p.to_string_lossy();
            debug!("Loading release directory: {}", p_str);
            let r = ReleaseStore::load(StoreRoot::new(p.clone())?, &bar_release_loading);
            if r.is_ok() {
                bar_release_loading
                    .finish_with_message(format!("Loaded releases in {p_str} successfully"));
            } else {
                bar_release_loading
                    .finish_with_message(format!("Failed to load releases in {p_str}"));
            }
            r.map(Arc::new)
        })
        .collect::<Result<Vec<_>>>()?;

    drop(loading_span_guard);

    let (staging_store, staging_dir, submit_id) = {
        let bar_staging_loading = progressbars.bar()?;

        let (submit_id, p) = if let Some(staging_dir) =
            matches.get_one::<String>("staging_dir").map(PathBuf::from)
        {
            info!(
                parent: &loading_span,
                "Setting staging dir to {} for this run",
                staging_dir.display()
            );

            let uuid = staging_dir
                .file_name()
                .ok_or_else(|| anyhow!("Seems not to be a directory: {}", staging_dir.display()))?
                .to_owned()
                .into_string()
                .map_err(|_| anyhow!("Type conversion of staging dir name to UTF8 String"))
                .context("Parsing staging dir name to UUID")?;
            let uuid = Uuid::parse_str(&uuid)
                .context("Parsing directory name as UUID")
                .with_context(|| anyhow!("Seems not to be a submit UUID: {}", uuid))?;

            (uuid, staging_dir)
        } else {
            let submit_id = uuid::Uuid::new_v4();
            let staging_dir = config
                .staging_directory()
                .join(submit_id.hyphenated().to_string());

            (submit_id, staging_dir)
        };

        if !p.is_dir() {
            tokio::fs::create_dir_all(&p)
                .instrument(
                    tracing::trace_span!(parent: &loading_span, "Creating directories", path = ?p),
                )
                .await?;
        }

        debug!(parent: &loading_span, "Loading staging directory: {}", p.display());
        let r = StagingStore::load(StoreRoot::new(p.clone())?, &bar_staging_loading);
        if r.is_ok() {
            bar_staging_loading.finish_with_message("Loaded staging successfully");
        } else {
            bar_staging_loading.finish_with_message("Failed to load staging");
        }
        r.map(RwLock::new)
            .map(Arc::new)
            .map(|store| (store, p, submit_id))?
    };

    let dag = {
        let bar_tree_building = progressbars.bar()?;
        let condition_data = ConditionData {
            image_name: Some(&image_name),
            env: &additional_env,
        };

        let dag = Dag::for_root_package(
            package.clone(),
            &repo,
            Some(&bar_tree_building),
            &condition_data,
        )?;
        bar_tree_building.finish_with_message("Finished loading Dag");
        dag
    };

    let source_cache = SourceCache::new(config.source_cache_root().clone());

    if matches.get_flag("no_verification") {
        warn!(parent: &loading_span, "No hash verification will be performed");
    } else {
        crate::commands::source::verify_impl(
            dag.all_packages().into_iter(),
            &source_cache,
            &progressbars,
        )
        .instrument(tracing::trace_span!(parent: &loading_span, "verify source hashes"))
        .await?;
    }

    // linting the package scripts
    if matches.get_flag("no_lint") {
        warn!(parent: &loading_span, "No script linting will be performed!");
    } else if let Some(linter) = crate::ui::find_linter_command(repo_root, config)? {
        let all_packages = dag.all_packages();
        let bar = progressbars.bar()?;
        bar.set_length(all_packages.len() as u64);
        bar.set_message("Linting package scripts...");

        let iter = all_packages.into_iter();
        crate::commands::util::lint_packages(iter, &linter, config, bar).await?;
    } else {
        warn!(parent: &loading_span, "No linter set in configuration, no script linting will be performed!");
    } // linting

    dag.all_packages()
        .into_iter()
        .map(|pkg| {
            if let Some(allowlist) = pkg.allowed_images() {
                if !allowlist.contains(&image_name) {
                    return Err(anyhow!(
                        "Package {} {} is only allowed on: {}",
                        pkg.name(),
                        pkg.version(),
                        allowlist.iter().join(", ")
                    ));
                }
            }

            if let Some(deniedlist) = pkg.denied_images() {
                if deniedlist.iter().any(|denied| image_name == *denied) {
                    return Err(anyhow!(
                        "Package {} {} is not allowed to be built on {}",
                        pkg.name(),
                        pkg.version(),
                        image_name
                    ));
                }
            }

            Ok(())
        })
        .collect::<Result<Vec<()>>>()?;

    drop(loading_span);
    let submit_span = tracing::debug_span!(parent: &command_span, "submit");

    trace!(parent: &submit_span, "Setting up database jobs for Package, GitHash, Image");
    let db_package = async { Package::create_or_fetch(&mut database_pool.get().unwrap(), package) };
    let db_githash =
        async { GitHash::create_or_fetch(&mut database_pool.get().unwrap(), &hash_str) };
    let db_image = async { Image::create_or_fetch(&mut database_pool.get().unwrap(), &image_name) };
    let db_envs = async {
        additional_env
            .clone()
            .into_iter()
            .map(|(k, v)| async {
                let k: EnvironmentVariableName = k; // hack to work around move semantics
                let v: String = v; // hack to work around move semantics
                EnvVar::create_or_fetch(&mut database_pool.get().unwrap(), &k, &v)
            })
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<Result<Vec<EnvVar>>>()
            .await
    };

    trace!(parent: &submit_span, "Running database jobs for Package, GitHash, Image");
    let (db_package, db_githash, db_image, db_envs) =
        tokio::join!(db_package, db_githash, db_image, db_envs);

    let (db_package, db_githash, db_image, _) = (db_package?, db_githash?, db_image?, db_envs?);

    trace!(parent: &submit_span, "Database jobs for Package, GitHash, Image finished successfully");
    trace!(parent: &submit_span, "Creating Submit in database");
    let submit = Submit::create(
        &mut database_pool.get().unwrap(),
        &now,
        &submit_id,
        &db_image,
        &db_package,
        &db_githash,
    )?;
    trace!(
        parent: &submit_span,
        "Creating Submit in database finished successfully: {:?}",
        submit
    );

    {
        let out = std::io::stdout();
        let mut outlock = out.lock();

        #[inline]
        fn mkgreen<T: ToString>(t: &T) -> colored::ColoredString {
            t.to_string().green()
        }

        writeln!(outlock, "Starting submit: {}", mkgreen(&submit_id))?;
        writeln!(outlock, "Started at:      {}", mkgreen(&now))?;
        writeln!(outlock, "On Image:        {}", mkgreen(&db_image.name))?;
        writeln!(
            outlock,
            "For Package:     {p} {v}",
            p = mkgreen(&db_package.name),
            v = mkgreen(&db_package.version)
        )?;
        writeln!(outlock, "On repo hash:    {}", mkgreen(&db_githash.hash))?;
    }

    trace!(parent: &submit_span, "Setting up job sets");
    let resources: Vec<JobResource> = additional_env.into_iter().map(JobResource::from).collect();
    let jobdag =
        crate::job::Dag::from_package_dag(dag, shebang, image_name, phases.clone(), resources);
    trace!(parent: &submit_span, "Setting up job sets finished successfully");
    drop(submit_span);

    let build_span = tracing::debug_span!(parent: &command_span, "build");

    trace!(parent: &build_span, "Setting up Orchestrator");
    let orch = OrchestratorSetup::builder()
        .progress_generator(progressbars)
        .endpoint_config(endpoint_configurations)
        .staging_store(staging_store)
        .release_stores(release_stores)
        .database(database_pool.clone())
        .source_cache(source_cache)
        .submit(submit)
        .log_dir(if matches.get_flag("write-log-file") {
            Some(config.log_dir().clone())
        } else {
            None
        })
        .jobdag(jobdag)
        .config(config)
        .repository(git_repo)
        .build()
        .setup()
        .instrument(build_span.clone())
        .await?;

    info!(parent: &build_span, "Running orchestrator...");
    let mut artifacts = vec![];
    let errors = orch.run(&mut artifacts).instrument(build_span).await?;
    let out = std::io::stdout();
    let mut outlock = out.lock();

    if !artifacts.is_empty() {
        writeln!(outlock, "Packages created:")?;
    }
    artifacts.into_iter().try_for_each(|artifact_path| {
        writeln!(outlock, "{}", staging_dir.join(artifact_path).display()).map_err(Error::from)
    })?;

    let mut had_error = false;
    for (job_uuid, error) in errors {
        had_error = true;
        for cause in error.chain() {
            writeln!(outlock, "{}: {}", "[ERROR]".red(), cause)?;
        }

        let data = schema::jobs::table
            .filter(schema::jobs::dsl::uuid.eq(job_uuid))
            .inner_join(schema::packages::table)
            .first::<(Job, Package)>(&mut *database_pool.get().unwrap())?;

        let number_log_lines = *config.build_error_lines();
        writeln!(
            outlock,
            "Last {} lines of Job {}",
            number_log_lines,
            job_uuid.to_string().red()
        )?;
        writeln!(
            outlock,
            "for package {} {}\n\n",
            data.1.name.to_string().red(),
            data.1.version.to_string().red()
        )?;

        let mut last_phase = None;
        let mut error_catched = false;
        let lines = crate::log::ParsedLog::from_str(&data.0.log_text)?
            .into_iter()
            .map(|line_item| {
                if let LogItem::CurrentPhase(ref p) = line_item {
                    if !error_catched {
                        last_phase = Some(p.clone());
                    }
                }

                if let LogItem::State(_) = line_item {
                    error_catched = true;
                }

                line_item.display().map(|d| d.to_string())
            })
            .collect::<Result<Vec<_>>>()?;

        lines
            .iter()
            .enumerate()
            .skip({
                if lines.len() > number_log_lines {
                    lines.len() - number_log_lines
                } else {
                    lines.len()
                }
            })
            .try_for_each(|(i, line)| {
                let lineno = format!("{i:>4} | ").bright_black();
                writeln!(outlock, "{lineno}{line}").map_err(Error::from)
            })?;

        writeln!(outlock, "\n\n")?;
        if error_catched {
            if let Some(last_phase) = last_phase {
                writeln!(outlock, "\tJob errored in Phase '{last_phase}'")?;
            }
            writeln!(outlock, "\n\n")?;
        } else {
            writeln!(
                outlock,
                "{}",
                "Error seems not to be caused by packaging script.".red()
            )?;
        }
    }

    if had_error {
        Err(anyhow!("One or multiple errors during build"))
    } else {
        Ok(())
    }
}
