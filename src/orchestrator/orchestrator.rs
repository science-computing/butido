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
    database: PgConnection,
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
        let scheduler = EndpointScheduler::setup(self.endpoint_config, self.staging_store.clone()).await?;

        Ok(Orchestrator {
            progress_generator:    self.progress_generator,
            scheduler:             scheduler,
            staging_store:         self.staging_store,
            release_store:         self.release_store,
            source_cache:          self.source_cache,
            jobsets:               self.jobsets,
            database:              self.database,
            file_log_sink_factory: self.file_log_sink_factory,
        })
    }
}

impl Orchestrator {

    pub async fn run(self) -> Result<Vec<PathBuf>> {
        use tokio::stream::StreamExt;

        let mut report_result = vec![];
        let number_of_jobsets = self.jobsets.len();
        let database = Arc::new(self.database);

        for (i, jobset) in self.jobsets.into_iter().enumerate() {
            // create a multi-bar for showing the overall jobset status as well as one bar per
            // running job.
            let jobset_bar = indicatif::MultiProgress::default();

            // Create a "overview bar", which shows the progress of all jobs of the jobset combined
            let jobset_overview_bar = jobset_bar.add({
                self.progress_generator.jobset_bar(i + 1, number_of_jobsets, jobset.len())
            });

            let merged_store = MergedStores::new(self.release_store.clone(), self.staging_store.clone());

            let (results, logs) = { // run the jobs in the set
                let unordered_results   = futures::stream::FuturesUnordered::new();
                let unordered_receivers = futures::stream::FuturesUnordered::new();
                for runnable in jobset.into_runables(&merged_store, &self.source_cache) {
                    let runnable = runnable?;
                    let job_id = runnable.uuid().clone();
                    trace!("Runnable {} for package {}", job_id, runnable.package().name());
                    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<LogItem>();

                    let jobhandle = self.scheduler.schedule_job(runnable, sender).await?;
                    trace!("Jobhandle -> {:?}", jobhandle);

                    // clone the bar here, so we can give a handle to the async result fetcher closure
                    // where we tick() it as soon as the job returns the result (= is finished)
                    let bar = jobset_overview_bar.clone();

                    unordered_results.push(async move {
                        let r = jobhandle.get_result().await;
                        trace!("Found result in job {}: {:?}", job_id, r);
                        bar.tick();
                        r
                    });
                    unordered_receivers.push(async move {
                        (job_id, receiver)
                    });
                }

                (unordered_results.collect::<Result<Vec<_>>>(), unordered_receivers.collect::<Vec<_>>())
            };

            let (results, logs) = tokio::join!(results, logs);
            // TODO: Use logs.

            {
                let log_processing_results = futures::stream::FuturesUnordered::new();
                for (job_id, log) in logs {
                    let bar = jobset_bar.add(self.progress_generator.job_bar(&job_id));
                    let db = database.clone();
                    log_processing_results.push(async move {
                        LogReceiver {
                            job_id,
                            log,
                            bar,
                            db,
                        }.join()
                    });
                }

                let _ = log_processing_results.collect::<Vec<_>>().await;
            }

            let results = results?
                .into_iter()
                .flatten()
                .collect::<Vec<PathBuf>>();

            let _ = jobset_overview_bar.finish();
            let _ = jobset_bar.join()?;

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

        Ok(report_result)
    }

}

struct LogReceiver {
    job_id: Uuid,
    log: UnboundedReceiver<LogItem>,
    bar: ProgressBar,
    db: Arc<PgConnection>,
}

impl LogReceiver {
    async fn join(mut self) -> Result<()> {
        let mut success = None;
        while let Some(logitem) = self.log.recv().await {
            match logitem {
                LogItem::Line(_) => {
                    // ignore
                },
                LogItem::Progress(u) => {
                    self.bar.set_position(u as u64);
                },
                LogItem::CurrentPhase(phasename) => {
                    self.bar.set_message(&format!("{} Phase: {}", self.job_id, phasename));
                },
                LogItem::State(Ok(s)) => {
                    self.bar.set_message(&format!("{} State Ok: {}", self.job_id, s));
                    success = Some(true);
                },
                LogItem::State(Err(e)) => {
                    self.bar.set_message(&format!("{} State Err: {}", self.job_id, e));
                    success = Some(false);
                },
            }
        }

        match success {
            Some(true) => self.bar.finish_with_message(&format!("{} finished successfully", self.job_id)),
            Some(false) => self.bar.finish_with_message(&format!("{} finished with error", self.job_id)),
            None => self.bar.finish_with_message(&format!("{} finished", self.job_id)),
        }

        Ok(())
    }
}

