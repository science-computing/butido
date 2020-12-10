use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use diesel::PgConnection;
use log::trace;
use tokio::sync::RwLock;
use typed_builder::TypedBuilder;

use crate::config::Configuration;
use crate::db::models::Artifact;
use crate::db::models::Submit;
use crate::endpoint::EndpointConfiguration;
use crate::endpoint::EndpointScheduler;
use crate::filestore::MergedStores;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::job::JobSet;
use crate::job::RunnableJob;
use crate::source::SourceCache;
use crate::util::progress::ProgressBars;

pub struct Orchestrator<'a> {
    scheduler: EndpointScheduler,
    progress_generator: ProgressBars,
    merged_stores: MergedStores,
    source_cache: SourceCache,
    jobsets: Vec<JobSet>,
    config: &'a Configuration,
}

#[derive(TypedBuilder)]
pub struct OrchestratorSetup<'a> {
    progress_generator: ProgressBars,
    endpoint_config: Vec<EndpointConfiguration>,
    staging_store: Arc<RwLock<StagingStore>>,
    release_store: Arc<RwLock<ReleaseStore>>,
    source_cache: SourceCache,
    jobsets: Vec<JobSet>,
    database: PgConnection,
    submit: Submit,
    log_dir: Option<PathBuf>,
    config: &'a Configuration,
}

impl<'a> OrchestratorSetup<'a> {
    pub async fn setup(self) -> Result<Orchestrator<'a>> {
        let db = Arc::new(self.database);
        let scheduler = EndpointScheduler::setup(self.endpoint_config, self.staging_store.clone(), db, self.submit.clone(), self.log_dir).await?;

        Ok(Orchestrator {
            scheduler:     scheduler,
            progress_generator: self.progress_generator,
            merged_stores: MergedStores::new(self.release_store, self.staging_store),
            source_cache:  self.source_cache,
            jobsets:       self.jobsets,
            config:        self.config,
        })
    }
}

impl<'a> Orchestrator<'a> {

    pub async fn run(self, output: &mut Vec<Artifact>) -> Result<Vec<anyhow::Error>> {
        for jobset in self.jobsets.into_iter() {
            let errs = Self::run_jobset(&self.scheduler,
                &self.merged_stores,
                &self.source_cache,
                &self.config,
                &self.progress_generator,
                jobset,
                output)
                .await?;

            if !errs.is_empty() {
                return Ok(errs)
            }
        }

        Ok(vec![])
    }

    async fn run_jobset(
        scheduler: &EndpointScheduler,
        merged_store: &MergedStores,
        source_cache: &SourceCache,
        config: &Configuration,
        progress_generator: &ProgressBars,
        jobset: JobSet,
        output: &mut Vec<Artifact>)
        -> Result<Vec<anyhow::Error>>
    {
        use tokio::stream::StreamExt;

        let multibar = Arc::new(indicatif::MultiProgress::new());
        let results = jobset // run the jobs in the set
            .into_runables(&merged_store, source_cache, config)
            .await?
            .into_iter()
            .map(|runnable| {
                let bar = multibar.add(progress_generator.bar());
                Self::run_runnable(runnable, scheduler, bar)
            })
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<Vec<Result<Vec<Artifact>>>>();

        let multibar_block = tokio::task::spawn_blocking(move || multibar.join());

        let (results, barres) = tokio::join!(results, multibar_block);
        let _ = barres?;
        let (okays, errors): (Vec<_>, Vec<_>) = results
            .into_iter()
            .inspect(|e| trace!("Processing result from jobset run: {:?}", e))
            .partition(|e| e.is_ok());

        let results = okays.into_iter().filter_map(Result::ok).flatten().collect::<Vec<Artifact>>();

        { // check if all paths that were written are actually there in the staging store
            let staging_store_lock = merged_store.staging().read().await;

            trace!("Checking {} results...", results.len());
            for artifact in results.iter() {
                let a_path = artifact.path_buf();
                trace!("Checking path: {}", a_path.display());
                if !staging_store_lock.path_exists_in_store_root(&a_path) {
                    return Err(anyhow!("Result path {} is missing from staging store", a_path.display()))
                        .with_context(|| anyhow!("Should be: {}/{}", staging_store_lock.root_path().display(), a_path.display()))
                        .map_err(Error::from)
                }
            }

        }

        let mut results = results; // rebind
        output.append(&mut results);
        Ok(errors.into_iter().filter_map(Result::err).collect())
    }

    async fn run_runnable(runnable: RunnableJob, scheduler: &EndpointScheduler, bar: indicatif::ProgressBar) -> Result<Vec<Artifact>> {
        let job_id = runnable.uuid().clone();
        trace!("Runnable {} for package {}", job_id, runnable.package().name());

        let jobhandle = scheduler.schedule_job(runnable, bar).await?;
        trace!("Jobhandle -> {:?}", jobhandle);

        let r = jobhandle.run().await;
        trace!("Found result in job {}: {:?}", job_id, r);
        r
    }

}

