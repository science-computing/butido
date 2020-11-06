use std::ops::DerefMut;
use std::sync::Arc;
use std::sync::RwLock;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Result;
use tokio::stream::StreamExt;
use tokio::sync::mpsc::UnboundedSender;

use crate::endpoint::Endpoint;
use crate::endpoint::EndpointConfiguration;
use crate::job::JobSet;
use crate::job::RunnableJob;
use crate::log::LogItem;
use crate::filestore::StagingStore;

pub struct EndpointScheduler {
    endpoints: Vec<Arc<RwLock<Endpoint>>>,

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

    async fn setup_endpoints(endpoints: Vec<EndpointConfiguration>) -> Result<Vec<Arc<RwLock<Endpoint>>>> {
        use futures::FutureExt;

        let unordered = futures::stream::FuturesUnordered::new();

        for cfg in endpoints.into_iter() {
            unordered.push({
                Endpoint::setup(cfg)
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
    pub async fn schedule_job(&self, job: RunnableJob, sender: UnboundedSender<LogItem>) -> Result<JobHandle> {
        let endpoint = self.select_free_endpoint().await?;

        Ok(JobHandle {
            endpoint, job, sender,
            staging_store: self.staging_store.clone()
        })
    }

    async fn select_free_endpoint(&self) -> Result<Arc<RwLock<Endpoint>>> {
        use itertools::Itertools;
        use futures::FutureExt;

        loop {
            let unordered = futures::stream::FuturesUnordered::new();
            for ep in self.endpoints.iter().cloned() {
                unordered.push(async move {
                    let wl = ep.write().map_err(|_| anyhow!("Lock poisoned"))?;
                    wl.number_of_running_containers().await.map(|u| (u, ep.clone()))
                });
            }

            let endpoints = unordered.collect::<Result<Vec<_>>>().await?;

            if let Some(endpoint) = endpoints
                .iter()
                .sorted_by(|tpla, tplb| tpla.0.cmp(&tplb.0))
                .map(|tpl| tpl.1.clone())
                .next()
            {
                return Ok(endpoint)
            }
        }
    }

}

#[derive(Debug)]
pub struct JobHandle {
    endpoint: Arc<RwLock<Endpoint>>,
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

        Ok(res)
    }

}

