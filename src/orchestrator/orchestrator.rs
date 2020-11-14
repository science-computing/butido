use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use diesel::PgConnection;
use typed_builder::TypedBuilder;
use uuid::Uuid;
use tokio::sync::mpsc::UnboundedReceiver;
use indicatif::ProgressBar;

use crate::db::models::Submit;
use crate::endpoint::EndpointConfiguration;
use crate::endpoint::EndpointScheduler;
use crate::filestore::MergedStores;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::job::JobSet;
use crate::log::FileLogSinkFactory;
use crate::log::LogItem;
use crate::source::SourceCache;
use crate::util::progress::ProgressBars;

pub struct Orchestrator {
    progress_generator: ProgressBars,
    scheduler: EndpointScheduler,
    staging_store: Arc<RwLock<StagingStore>>,
    release_store: Arc<RwLock<ReleaseStore>>,
    source_cache: SourceCache,
    jobsets: Vec<JobSet>,
    database: Arc<PgConnection>,
    file_log_sink_factory: Option<FileLogSinkFactory>,
}

#[derive(TypedBuilder)]
pub struct OrchestratorSetup {
    progress_generator: ProgressBars,
    endpoint_config: Vec<EndpointConfiguration>,
    staging_store: Arc<RwLock<StagingStore>>,
    release_store: Arc<RwLock<ReleaseStore>>,
    source_cache: SourceCache,
    jobsets: Vec<JobSet>,
    database: PgConnection,
    submit: Submit,
    file_log_sink_factory: Option<FileLogSinkFactory>,
}

impl OrchestratorSetup {
    pub async fn setup(self) -> Result<Orchestrator> {
        let db = Arc::new(self.database);
        let scheduler = EndpointScheduler::setup(self.endpoint_config, self.staging_store.clone(), db.clone(), self.progress_generator.clone(), self.submit.clone()).await?;

        Ok(Orchestrator {
            progress_generator:    self.progress_generator,
            scheduler:             scheduler,
            staging_store:         self.staging_store,
            release_store:         self.release_store,
            source_cache:          self.source_cache,
            jobsets:               self.jobsets,
            database:              db,
            file_log_sink_factory: self.file_log_sink_factory,
        })
    }
}

impl Orchestrator {

    pub async fn run(self) -> Result<Vec<PathBuf>> {
        use tokio::stream::StreamExt;

        let mut report_result = vec![];
        let number_of_jobsets = self.jobsets.len();
        let database = self.database;

        for (i, jobset) in self.jobsets.into_iter().enumerate() {
            let merged_store = MergedStores::new(self.release_store.clone(), self.staging_store.clone());

            let results = { // run the jobs in the set
                let unordered_results   = futures::stream::FuturesUnordered::new();
                for runnable in jobset.into_runables(&merged_store, &self.source_cache) {
                    let runnable = runnable?;
                    let job_id = runnable.uuid().clone();
                    trace!("Runnable {} for package {}", job_id, runnable.package().name());

                    let jobhandle = self.scheduler.schedule_job(runnable).await?;
                    trace!("Jobhandle -> {:?}", jobhandle);

                    // clone the bar here, so we can give a handle to the async result fetcher closure
                    // where we tick() it as soon as the job returns the result (= is finished)
                    unordered_results.push(async move {
                        let r = jobhandle.run().await;
                        trace!("Found result in job {}: {:?}", job_id, r);
                        r
                    });
                }

                unordered_results.collect::<Result<Vec<_>>>()
            };

            let results = results.await?
                .into_iter()
                .flatten()
                .collect::<Vec<PathBuf>>();

            { // check if all paths that were written are actually there in the staging store
                let staging_store_lock = self.staging_store
                    .read()
                    .map_err(|_| anyhow!("Lock Poisoned"))?;

                trace!("Checking {} results...", results.len());
                for path in results.iter() {
                    trace!("Checking path: {}", path.display());
                    if !staging_store_lock.path_exists_in_store_root(&path) {
                        return Err(anyhow!("Result path {} is missing from staging store", path.display()))
                            .with_context(|| anyhow!("Should be: {}/{}", staging_store_lock.root_path().display(), path.display()))
                            .map_err(Error::from)
                    }
                }

            }

            let mut results = results; // rebind!
            report_result.append(&mut results);
        }

        self.scheduler.shutdown()?;
        Ok(report_result)
    }

}

