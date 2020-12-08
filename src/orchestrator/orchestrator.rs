use std::io::Write;
use std::path::PathBuf;
use std::result::Result as RResult;
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
use crate::endpoint::ContainerError;
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
    staging_store: Arc<RwLock<StagingStore>>,
    release_store: Arc<RwLock<ReleaseStore>>,
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
        let scheduler = EndpointScheduler::setup(self.endpoint_config, self.staging_store.clone(), db, self.progress_generator, self.submit.clone(), self.log_dir).await?;

        Ok(Orchestrator {
            scheduler:             scheduler,
            staging_store:         self.staging_store,
            release_store:         self.release_store,
            source_cache:          self.source_cache,
            jobsets:               self.jobsets,
            config:                self.config,
        })
    }
}

impl<'a> Orchestrator<'a> {

    pub async fn run(self) -> Result<Vec<Artifact>> {
        let mut report_result = vec![];
        let scheduler = self.scheduler; // moved here because of partial-move semantics
        let merged_store = MergedStores::new(self.release_store.clone(), self.staging_store.clone());

        for jobset in self.jobsets.into_iter() {
            let mut results = Self::run_jobset(&scheduler,
                &merged_store,
                &self.source_cache,
                &self.config,
                jobset)
                .await?;

            report_result.append(&mut results);
        }

        Ok(report_result)
    }

    async fn run_jobset(
        scheduler: &EndpointScheduler,
        merged_store: &MergedStores,
        source_cache: &SourceCache,
        config: &Configuration,
        jobset: JobSet)
        -> Result<Vec<Artifact>>
    {
        use tokio::stream::StreamExt;

        let multibar = Arc::new(indicatif::MultiProgress::new());
        let results = jobset // run the jobs in the set
            .into_runables(&merged_store, source_cache, config)
            .await?
            .into_iter()
            .map(|runnable| {
                let multibar = multibar.clone();

                async {
                    Self::run_runnable(multibar, runnable, scheduler).await
                }
            })
            .collect::<futures::stream::FuturesUnordered<_>>()
            .collect::<Vec<RResult<Vec<Artifact>, ContainerError>>>();

        let multibar_block = tokio::task::spawn_blocking(move || multibar.join());

        let (results, barres) = tokio::join!(results, multibar_block);
        let _ = barres?;
        let (okays, errors): (Vec<_>, Vec<_>) = results
            .into_iter()
            .inspect(|e| trace!("Processing result from jobset run: {:?}", e))
            .partition(|e| e.is_ok());

        let results = okays.into_iter().filter_map(Result::ok).flatten().collect::<Vec<Artifact>>();

        {
            let mut out = std::io::stderr();
            for error in errors {
                if let Err(e) = error {
                    if let Some(expl) = e.explain_container_error() {
                        writeln!(out, "{}", expl)?;
                    }
                }
            }
        }

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

        Ok(results)
    }

    async fn run_runnable(multibar: Arc<indicatif::MultiProgress>, runnable: RunnableJob, scheduler: &EndpointScheduler)
        -> RResult<Vec<Artifact>, ContainerError>
    {
        let job_id = runnable.uuid().clone();
        trace!("Runnable {} for package {}", job_id, runnable.package().name());

        let jobhandle = scheduler.schedule_job(runnable, multibar).await?;
        trace!("Jobhandle -> {:?}", jobhandle);

        let r = jobhandle.run().await;
        trace!("Found result in job {}: {:?}", job_id, r);
        r
    }

}

