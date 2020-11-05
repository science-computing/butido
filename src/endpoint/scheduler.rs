use std::ops::DerefMut;
use std::sync::Arc;
use std::sync::RwLock;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Result;
use tokio::stream::StreamExt;
use tokio::sync::mpsc::UnboundedSender;

use crate::endpoint::ConfiguredEndpoint;
use crate::endpoint::EndpointConfiguration;
use crate::job::JobSet;
use crate::job::RunnableJob;
use crate::log::LogItem;
use crate::filestore::StagingStore;

pub struct EndpointScheduler {
    endpoints: Vec<Arc<RwLock<ConfiguredEndpoint>>>,

    staging_store: Arc<RwLock<StagingStore>>,
}

impl EndpointScheduler {

    pub async fn setup(endpoints: Vec<EndpointConfiguration>, staging_store: Arc<RwLock<StagingStore>>) -> Result<Self> {
        let endpoints = Self::setup_endpoints(endpoints).await?;

        Ok(EndpointScheduler {
            endpoints,
            staging_store,
        })
    }

    async fn setup_endpoints(endpoints: Vec<EndpointConfiguration>) -> Result<Vec<Arc<RwLock<ConfiguredEndpoint>>>> {
        use futures::FutureExt;

        let unordered = futures::stream::FuturesUnordered::new();

        for cfg in endpoints.into_iter() {
            unordered.push({
                ConfiguredEndpoint::setup(cfg)
                    .map(|r_ep| {
                        r_ep.map(RwLock::new)
                            .map(Arc::new)
                    })
            });
        }

        unordered.collect().await
    }

    /// Schedule a Job
    ///
    /// # Warning
    ///
    /// This function blocks as long as there is no free endpoint available!
    pub fn schedule_job(&self, job: RunnableJob, sender: UnboundedSender<LogItem>) -> Result<JobHandle> {
        let endpoint = self.select_free_endpoint()?;

        Ok(JobHandle {
            endpoint, job, sender,
            staging_store: self.staging_store.clone()
        })
    }

    fn select_free_endpoint(&self) -> Result<Arc<RwLock<ConfiguredEndpoint>>> {
        loop {
            if let Some(e) = self
                .endpoints
                .iter()
                .filter_map(|endpoint| {
                    match endpoint.write() {
                        Err(e) => Some(Err(anyhow!("Lock poisoned"))),
                        Ok(mut ep) => {
                            if ep.num_current_jobs() < ep.num_max_jobs() {
                                ep.inc_current_jobs();
                                Some(Ok(endpoint.clone()))
                            } else {
                                None
                            }
                        }
                    }
                })
                .next()
            {
                return e
            }
        }
    }

}

pub struct JobHandle {
    endpoint: Arc<RwLock<ConfiguredEndpoint>>,
    job: RunnableJob,
    sender: UnboundedSender<LogItem>,
    staging_store: Arc<RwLock<StagingStore>>,
}

impl JobHandle {
    pub async fn get_result(self) -> Result<Vec<PathBuf>> {
        let res = self.endpoint
            .read()
            .map_err(|_| anyhow!("Lock poisoned"))?
            .run_job(self.job, self.sender, self.staging_store)
            .await?;

        {
            self.endpoint
                .write()
                .map_err(|_| anyhow!("Lock poisoned"))?
                .dec_current_jobs();
        }

        Ok(res)
    }

}

