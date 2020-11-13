use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use diesel::PgConnection;
use futures::FutureExt;
use indicatif::ProgressBar;
use itertools::Itertools;
use tokio::stream::StreamExt;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use crate::endpoint::Endpoint;
use crate::endpoint::EndpointConfiguration;
use crate::filestore::StagingStore;
use crate::job::RunnableJob;
use crate::log::LogItem;
use crate::util::progress::ProgressBars;

pub struct EndpointScheduler {
    endpoints: Vec<Arc<RwLock<Endpoint>>>,

    staging_store: Arc<RwLock<StagingStore>>,
    db: Arc<PgConnection>,
    progressbars: ProgressBars,
    multibar: indicatif::MultiProgress,
}

impl EndpointScheduler {

    pub async fn setup(endpoints: Vec<EndpointConfiguration>, staging_store: Arc<RwLock<StagingStore>>, db: Arc<PgConnection>, progressbars: ProgressBars) -> Result<Self> {
        let endpoints = Self::setup_endpoints(endpoints).await?;

        Ok(EndpointScheduler {
            endpoints,
            staging_store,
            db,
            progressbars,
            multibar: indicatif::MultiProgress::new(),
        })
    }

    async fn setup_endpoints(endpoints: Vec<EndpointConfiguration>) -> Result<Vec<Arc<RwLock<Endpoint>>>> {
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
    pub async fn schedule_job(&self, job: RunnableJob) -> Result<JobHandle> {
        let endpoint = self.select_free_endpoint().await?;

        Ok(JobHandle {
            bar: self.multibar.add(self.progressbars.job_bar(job.uuid())),
            endpoint,
            job,
            staging_store: self.staging_store.clone(),
            db: self.db.clone(),
        })
    }

    async fn select_free_endpoint(&self) -> Result<Arc<RwLock<Endpoint>>> {
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

pub struct JobHandle {
    endpoint: Arc<RwLock<Endpoint>>,
    job: RunnableJob,
    bar: ProgressBar,
    db: Arc<PgConnection>,
    staging_store: Arc<RwLock<StagingStore>>,
}

impl std::fmt::Debug for JobHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "JobHandle ( job: {} )", self.job.uuid())
    }
}

impl JobHandle {
    pub async fn run(self) -> Result<Vec<PathBuf>> {
        let (log_sender, log_receiver) = tokio::sync::mpsc::unbounded_channel::<LogItem>();
        let ep = self.endpoint
            .read()
            .map_err(|_| anyhow!("Lock poisoned"))?;

        let job_id = self.job.uuid().clone();
        trace!("Running on Job {} on Endpoint {}", job_id, ep.name());
        let res = ep
            .run_job(self.job, log_sender, self.staging_store);

        let logres = LogReceiver {
            job_id, 
            log_receiver, 
            bar: self.bar,
            db: self.db,
        }.join();

        let (res, logres) = tokio::join!(res, logres);

        trace!("Found result for job {}: {:?}", job_id, res);
        logres.with_context(|| anyhow!("Collecting logs for job on '{}'", ep.name()))?;
        let (paths, container_hash) = res.with_context(|| anyhow!("Running job on '{}'", ep.name()))?;


        Ok(paths)
    }

}

struct LogReceiver {
    job_id: Uuid,
    log_receiver: UnboundedReceiver<LogItem>,
    bar: ProgressBar,
    db: Arc<PgConnection>,
}

impl LogReceiver {
    async fn join(mut self) -> Result<()> {
        let mut success = None;
        while let Some(logitem) = self.log_receiver.recv().await {
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

