use std::sync::Arc;
use std::sync::RwLock;

use anyhow::Result;
use tokio::stream::StreamExt;
use tokio::sync::mpsc::UnboundedSender;

use crate::endpoint::EndpointManager;
use crate::endpoint::EndpointManagerConfiguration;
use crate::job::JobSet;
use crate::job::RunnableJob;
use crate::log::LogItem;
use crate::filestore::StagingStore;

pub struct EndpointScheduler {
    jobsets: Vec<JobSet>,

    endpoints: Vec<EndpointManager>,

    staging_store: Arc<RwLock<StagingStore>>,
}

impl EndpointScheduler {

    pub async fn setup(jobsets: Vec<JobSet>, endpoints: Vec<EndpointManagerConfiguration>, staging_store: StagingStore) -> Result<Self> {
        let endpoints = Self::setup_endpoints(endpoints).await?;

        Ok(EndpointScheduler {
            jobsets,
            endpoints,
            staging_store: Arc::new(RwLock::new(staging_store)),
        })
    }

    async fn setup_endpoints(endpoints: Vec<EndpointManagerConfiguration>) -> Result<Vec<EndpointManager>> {
        let unordered = futures::stream::FuturesUnordered::new();

        for cfg in endpoints.into_iter() {
            unordered.push(EndpointManager::setup(cfg));
        }

        unordered.collect().await
    }

    /// Run a jobset on all endpoints
    ///
    /// TODO: This is a naive implementation that simple pushes the complete jobset to the
    /// available endpoints.
    ///
    /// It does not yet schedule like it is supposed to do.
    pub async fn run_jobset(&self, js: Vec<(RunnableJob, UnboundedSender<LogItem>)>) -> Result<()> {
        let unordered    = futures::stream::FuturesUnordered::new();
        let mut i: usize = 0;
        let mut iter     = js.into_iter();

        while let Some(next_job) = iter.next() {
            match self.endpoints.get(i) {
                None => {
                    i = 0;
                },

                Some(ep) => {
                    let ep = ep.clone();
                    unordered.push(async {
                        ep.run_job(next_job.0, next_job.1, self.staging_store.clone()).await
                    });
                }
            }

            i += 1;
        }

        unordered.collect().await
    }

}
