use std::path::PathBuf;
use std::sync::RwLock;
use std::sync::Arc;
use std::collections::BTreeMap;

use anyhow::anyhow;
use anyhow::Error;
use anyhow::Context;
use anyhow::Result;
use typed_builder::TypedBuilder;
use diesel::PgConnection;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use resiter::AndThen;

use crate::endpoint::EndpointConfiguration;
use crate::endpoint::EndpointScheduler;
use crate::job::JobSet;
use crate::job::RunnableJob;
use crate::log::LogItem;
use crate::filestore::StagingStore;
use crate::filestore::ReleaseStore;
use crate::log::FileLogSinkFactory;
use crate::log::LogSink;
use crate::db::models::Submit;
use crate::db::models::EnvVar;
use crate::job::JobResource;
use crate::filestore::MergedStores;

pub struct Orchestrator {
    scheduler: EndpointScheduler,
    staging_store: Arc<RwLock<StagingStore>>,
    release_store: Arc<RwLock<ReleaseStore>>,
    jobsets: Vec<JobSet>,
    database: PgConnection,
    file_log_sink_factory: Option<FileLogSinkFactory>,
}

#[derive(TypedBuilder)]
pub struct OrchestratorSetup {
    endpoint_config: Vec<EndpointConfiguration>,
    staging_store: Arc<RwLock<StagingStore>>,
    release_store: Arc<RwLock<ReleaseStore>>,
    jobsets: Vec<JobSet>,
    database: PgConnection,
    submit: Submit,
    file_log_sink_factory: Option<FileLogSinkFactory>,
}

impl OrchestratorSetup {
    pub async fn setup(self) -> Result<Orchestrator> {
        let scheduler = EndpointScheduler::setup(self.endpoint_config, self.staging_store.clone()).await?;

        Ok(Orchestrator {
            scheduler:             scheduler,
            staging_store:         self.staging_store,
            release_store:         self.release_store,
            jobsets:               self.jobsets,
            database:              self.database,
            file_log_sink_factory: self.file_log_sink_factory,
        })
    }
}

impl Orchestrator {

    pub async fn run(self) -> Result<()> {
        use tokio::stream::StreamExt;

        let _database = self.database;
        for jobset in self.jobsets.into_iter() {
            let merged_store = MergedStores::new(self.release_store.clone(), self.staging_store.clone());

            let results = { // run the jobs in the set
                let unordered = futures::stream::FuturesUnordered::new();
                for runnable in jobset.into_runables(&merged_store) {
                    let runnable = runnable?;
                    trace!("Runnable {} for package {}", runnable.uuid(), runnable.package().name());
                    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<LogItem>();

                    let jobhandle = self.scheduler.schedule_job(runnable, sender).await?;
                    trace!("Jobhandle -> {:?}", jobhandle);
                    unordered.push(async move {
                        jobhandle.get_result().await
                    });
                }

                unordered.collect::<Result<Vec<_>>>().await?
                    .into_iter()
                    .flatten()
                    .collect::<Vec<PathBuf>>()
            };

            { // check if all paths that were written are actually there in the staging store
                let staging_store_lock = self.staging_store
                    .read()
                    .map_err(|_| anyhow!("Lock Poisoned"))?;

                trace!("Checking results...");
                for path in results.iter() {
                    trace!("Checking path: {}", path.display());
                    if !staging_store_lock.path_exists_in_store_root(&path) {
                        return Err(anyhow!("Result path {} is missing from staging store", path.display()))
                            .with_context(|| anyhow!("Should be: {}/{}", staging_store_lock.root_path().display(), path.display()))
                            .map_err(Error::from)
                    }
                }
            }

            { // register all written paths to the store
                let mut staging_store_lock = self.staging_store
                    .write()
                    .map_err(|_| anyhow!("Lock Poisoned"))?;

                trace!("Loading results into staging store");
                for path in results.iter() {
                    trace!("Loading path: {}", path.display());
                    staging_store_lock.load_from_path(&path)
                        .context("Loading artifacts into staging store")?;
                }
            }
        }

        Ok(())
    }

}
