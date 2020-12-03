use std::io::Write;
use std::path::PathBuf;
use std::result::Result as RResult;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use diesel::PgConnection;
use indicatif::ProgressBar;
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedReceiver;
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::db::models::Submit;
use crate::endpoint::ContainerError;
use crate::endpoint::EndpointConfiguration;
use crate::endpoint::EndpointScheduler;
use crate::filestore::MergedStores;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::job::JobSet;
use crate::log::LogItem;
use crate::source::SourceCache;
use crate::util::progress::ProgressBars;

pub struct Orchestrator {
    scheduler: EndpointScheduler,
    staging_store: Arc<RwLock<StagingStore>>,
    release_store: Arc<RwLock<ReleaseStore>>,
    source_cache: SourceCache,
    jobsets: Vec<JobSet>,
    database: Arc<PgConnection>,
}

#[derive(TypedBuilder)]
pub struct OrchestratorSetup {
    progress_generator: ProgressBars,
    endpoint_config: Vec<EndpointConfiguration>,
    staging_store: Arc<RwLock<StagingStore>>,
    release_store: Arc<RwLock<ReleaseStore>>,
    source_cache: SourceCache,
    additional_env: Vec<(String, String)>,
    jobsets: Vec<JobSet>,
    database: PgConnection,
    submit: Submit,
    log_dir: Option<PathBuf>,
}

impl OrchestratorSetup {
    pub async fn setup(self) -> Result<Orchestrator> {
        let db = Arc::new(self.database);
        let scheduler = EndpointScheduler::setup(self.endpoint_config, self.staging_store.clone(), db.clone(), self.progress_generator, self.submit.clone(), self.log_dir, self.additional_env).await?;

        Ok(Orchestrator {
            scheduler:             scheduler,
            staging_store:         self.staging_store,
            release_store:         self.release_store,
            source_cache:          self.source_cache,
            jobsets:               self.jobsets,
            database:              db,
        })
    }
}

impl Orchestrator {

    pub async fn run(self) -> Result<Vec<PathBuf>> {
        use tokio::stream::StreamExt;

        let mut report_result = vec![];

        for jobset in self.jobsets.into_iter() {
            let merged_store = MergedStores::new(self.release_store.clone(), self.staging_store.clone());

            let multibar = Arc::new(indicatif::MultiProgress::new());

            let results = { // run the jobs in the set
                let unordered_results   = futures::stream::FuturesUnordered::new();
                for runnable in jobset.into_runables(&merged_store, &self.source_cache).await?.into_iter() {
                    let job_id = runnable.uuid().clone();
                    trace!("Runnable {} for package {}", job_id, runnable.package().name());

                    let jobhandle = self.scheduler.schedule_job(runnable, multibar.clone()).await?;
                    trace!("Jobhandle -> {:?}", jobhandle);

                    // clone the bar here, so we can give a handle to the async result fetcher closure
                    // where we tick() it as soon as the job returns the result (= is finished)
                    unordered_results.push(async move {
                        let r = jobhandle.run().await;
                        trace!("Found result in job {}: {:?}", job_id, r);
                        r
                    });
                }

                unordered_results.collect::<Vec<RResult<_, ContainerError>>>()
            };

            let multibar_block = tokio::task::spawn_blocking(move || multibar.join());

            let (results, barres) = tokio::join!(results, multibar_block);
            let _ = barres?;
            let (okays, errors): (Vec<_>, Vec<_>) = results
                .into_iter()
                .inspect(|e| trace!("Processing result from jobset run: {:?}", e))
                .partition(|e| e.is_ok());

            let results = okays.into_iter().filter_map(Result::ok).flatten().collect::<Vec<PathBuf>>();

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
                let staging_store_lock = self.staging_store.read().await;

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

