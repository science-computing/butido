use std::path::PathBuf;
use std::sync::RwLock;
use std::sync::Arc;

use anyhow::Result;
use typed_builder::TypedBuilder;
use diesel::PgConnection;

use crate::endpoint::EndpointManagerConfiguration;
use crate::endpoint::EndpointScheduler;
use crate::job::JobSet;
use crate::job::RunnableJob;
use crate::log::LogItem;
use crate::filestore::StagingStore;
use crate::filestore::ReleaseStore;

pub struct Orchestrator {
    scheduler: EndpointScheduler,
    staging_store: Arc<RwLock<StagingStore>>,
    release_store: Arc<RwLock<ReleaseStore>>,
    jobsets: Vec<JobSet>,
    database: PgConnection,
}

#[derive(TypedBuilder)]
pub struct OrchestratorSetup {
    ep_cfg: Vec<EndpointManagerConfiguration>,
    staging_store: Arc<RwLock<StagingStore>>,
    release_store: Arc<RwLock<ReleaseStore>>,
    jobsets: Vec<JobSet>,
    database: PgConnection,
}

impl OrchestratorSetup {
    pub async fn setup(self) -> Result<Orchestrator> {
        let scheduler = EndpointScheduler::setup(self.ep_cfg, self.staging_store.clone()).await?;

        Ok(Orchestrator {
            scheduler:      scheduler,
            staging_store:  self.staging_store,
            release_store:  self.release_store,
            jobsets:        self.jobsets,
            database:       self.database,
        })
    }
}

impl Orchestrator {

    pub async fn run(self) -> Result<()> {
        unimplemented!()
    }

}
