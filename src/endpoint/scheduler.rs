use std::sync::Arc;
use std::sync::RwLock;
use std::path::PathBuf;

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

}

