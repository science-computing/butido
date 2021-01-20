//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::anyhow;
use anyhow::Error;
use anyhow::Result;
use diesel::PgConnection;
use indicatif::ProgressBar;
use log::trace;
use tokio::sync::RwLock;
use tokio::stream::StreamExt;
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::config::Configuration;
use crate::db::models as dbmodels;
use crate::endpoint::EndpointConfiguration;
use crate::endpoint::EndpointScheduler;
use crate::filestore::Artifact;
use crate::filestore::MergedStores;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::job::JobDefinition;
use crate::job::RunnableJob;
use crate::job::Tree as JobTree;
use crate::source::SourceCache;
use crate::util::progress::ProgressBars;

pub struct Orchestrator<'a> {
    scheduler: EndpointScheduler,
    progress_generator: ProgressBars,
    merged_stores: MergedStores,
    source_cache: SourceCache,
    jobtree: JobTree,
    config: &'a Configuration,
    database: Arc<PgConnection>,
}

#[derive(TypedBuilder)]
pub struct OrchestratorSetup<'a> {
    progress_generator: ProgressBars,
    endpoint_config: Vec<EndpointConfiguration>,
    staging_store: Arc<RwLock<StagingStore>>,
    release_store: Arc<RwLock<ReleaseStore>>,
    source_cache: SourceCache,
    jobtree: JobTree,
    database: Arc<PgConnection>,
    submit: dbmodels::Submit,
    log_dir: Option<PathBuf>,
    config: &'a Configuration,
}

impl<'a> OrchestratorSetup<'a> {
    pub async fn setup(self) -> Result<Orchestrator<'a>> {
        let scheduler = EndpointScheduler::setup(
            self.endpoint_config,
            self.staging_store.clone(),
            self.database.clone(),
            self.submit.clone(),
            self.log_dir,
        )
        .await?;

        Ok(Orchestrator {
            scheduler,
            progress_generator: self.progress_generator,
            merged_stores: MergedStores::new(self.release_store, self.staging_store),
            source_cache: self.source_cache,
            jobtree: self.jobtree,
            config: self.config,
            database: self.database,
        })
    }
}

/// Helper type
///
/// Represents a result that came from the run of a job inside a container
///
/// It is either a list of artifacts (with their respective database artifact objects)
/// or a UUID and an Error object, where the UUID is the job UUID and the error is the
/// anyhow::Error that was issued.
type JobResult = std::result::Result<Vec<(Artifact, dbmodels::Artifact)>, (Uuid, Error)>;

impl<'a> Orchestrator<'a> {
    pub async fn run(self, output: &mut Vec<dbmodels::Artifact>) -> Result<Vec<(Uuid, Error)>> {
        let (results, errors) = self.run_tree().await?;
        output.extend(results.into_iter().map(|(_, dba)| dba));
        Ok(errors)
    }

    async fn run_tree(self) -> Result<(Vec<(Artifact, dbmodels::Artifact)>, Vec<(Uuid, Error)>)> {
        use futures::FutureExt;

        let mut already_built = vec![];
        let mut artifacts = vec![];
        let mut errors = vec![];

        loop {
            // loop{}
            //  until for all elements of self.jobtree, the uuid exists in already_built
            //
            //  for each element in jobtree
            //      where dependencies(element) all in already_built
            //      run_job_for(element)
            //
            //  for results from run_job_for calls
            //      remember UUID in already_built
            //      put built artifacts in artifacts
            //      if error, abort everything
            //
            //
            let multibar = Arc::new(indicatif::MultiProgress::new());
            let build_results = self.jobtree
                .inner()
                .iter()
                .filter(|(uuid, jobdef)| { // select all jobs where all dependencies are in `already_built`
                    trace!("Filtering job definition: {:?}", jobdef);
                    jobdef.dependencies.iter().all(|d| already_built.contains(d)) && !already_built.contains(uuid)
                })
                .map(|(uuid, jobdef)| {
                    trace!("Running job {}", uuid);
                    let bar = multibar.add(self.progress_generator.bar());
                    let uuid = uuid.clone();
                    self.run_job(jobdef, bar).map(move |r| (uuid, r))
                })
                .collect::<futures::stream::FuturesUnordered<_>>()
                .collect::<Vec<(_, Result<JobResult>)>>();

            let multibar_block = tokio::task::spawn_blocking(move || multibar.join());                        
            let (_, build_results) = tokio::join!(multibar_block, build_results);

            for (uuid, artifact_result) in build_results.into_iter() {
                already_built.push(uuid);

                match artifact_result {
                    Ok(Ok(mut arts)) => artifacts.append(&mut arts),
                    Ok(Err((uuid, e))) => { // error during job running
                        log::error!("Error for job {} = {}", uuid, e);
                        errors.push((uuid, e));
                    },

                    Err(e) => return Err(e), // error during container execution
                }
            }

            if !errors.is_empty() {
                break
            }

            // already_built.sort(); // TODO: optimization for binary search in
            // above and below contains() clause

            if self.jobtree.inner().iter().all(|(uuid, _)| already_built.contains(uuid)) {
                break
            }
        }

        Ok((artifacts, errors))
    }

    async fn run_job(&self, jobdef: &JobDefinition, bar: ProgressBar) -> Result<JobResult> {
        let dependency_artifacts = self.get_dependency_artifacts_for_jobs(&jobdef.dependencies).await?;
        bar.set_message("Preparing...");

        let runnable = RunnableJob::build_from_job(
            &jobdef.job,
            &self.source_cache,
            &self.config,
            dependency_artifacts)
            .await?;

        bar.set_message("Scheduling...");
        let job_uuid = jobdef.job.uuid().clone();
        match self.scheduler.schedule_job(runnable, bar).await?.run().await {
            Err(e) => return Ok(Err((job_uuid, e))),
            Ok(db_artifacts) => {
                db_artifacts.into_iter()
                    .map(|db_artifact| async {
                        trace!("Getting store Artifact for db Artifact: {:?}", db_artifact);
                        let art = self.get_store_artifact_for(&db_artifact).await?;
                        trace!("Store Artifact: {:?}", art);
                        Ok(Ok((art, db_artifact)))
                    })
                    .collect::<futures::stream::FuturesUnordered<_>>()
                    .collect::<Result<JobResult>>()
                    .await
            },
        }
    }

    /// Get all dependency artifacts for the job from the database
    ///
    /// Use the JobDefinition object and find all dependency outputs in the database
    async fn get_dependency_artifacts_for_jobs(&self, uuids: &[Uuid]) -> Result<Vec<Artifact>> {
        use crate::schema;
        use crate::diesel::ExpressionMethods;
        use crate::diesel::QueryDsl;
        use crate::diesel::RunQueryDsl;

        // Pseudo code:
        //
        // * return for uuid in uuids:
        //      self.database.get(job).get_artifacts()

        schema::artifacts::table
            .left_outer_join(schema::jobs::table)
            .filter(schema::jobs::uuid.eq_any(uuids))
            .select(schema::artifacts::all_columns)
            .load::<dbmodels::Artifact>(&*self.database)?
            .iter()
            .map(|dbart| self.get_store_artifact_for(dbart))
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect()
            .await
    }

    async fn get_store_artifact_for(&self, db_artifact: &dbmodels::Artifact) -> Result<Artifact> {
        let p = PathBuf::from(&db_artifact.path);
        self.merged_stores
            .get_artifact_by_path(&p)
            .await?
            .ok_or_else(|| {
                anyhow!("Artifact not found in {}", p.display())
            })
    }
}
