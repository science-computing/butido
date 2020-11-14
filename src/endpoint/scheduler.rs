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
    submit: crate::db::models::Submit,
}

impl EndpointScheduler {

    pub async fn setup(endpoints: Vec<EndpointConfiguration>, staging_store: Arc<RwLock<StagingStore>>, db: Arc<PgConnection>, progressbars: ProgressBars, submit: crate::db::models::Submit) -> Result<Self> {
        let endpoints = Self::setup_endpoints(endpoints).await?;

        Ok(EndpointScheduler {
            endpoints,
            staging_store,
            db,
            progressbars,
            multibar: indicatif::MultiProgress::new(),
            submit,
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
            submit: self.submit.clone(),
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
    submit: crate::db::models::Submit,
}

impl std::fmt::Debug for JobHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "JobHandle ( job: {} )", self.job.uuid())
    }
}

impl JobHandle {
    pub async fn run(self) -> Result<Vec<PathBuf>> {
        use crate::db::models as dbmodels;
        let (log_sender, log_receiver) = tokio::sync::mpsc::unbounded_channel::<LogItem>();
        let ep = self.endpoint
            .read()
            .map_err(|_| anyhow!("Lock poisoned"))?;

        let endpoint = dbmodels::Endpoint::create_or_fetch(&self.db, ep.name())?;
        let package  = dbmodels::Package::create_or_fetch(&self.db, self.job.package())?;
        let image    = dbmodels::Image::create_or_fetch(&self.db, self.job.image())?;

        let job_id = self.job.uuid().clone();
        trace!("Running on Job {} on Endpoint {}", job_id, ep.name());
        let res = ep
            .run_job(self.job, log_sender, self.staging_store);

        let logres = LogReceiver {
            job_id,
            log_receiver,
            bar: &self.bar,
            db: self.db.clone(),
        }.join();

        let (res, logres) = tokio::join!(res, logres);

        trace!("Found result for job {}: {:?}", job_id, res);
        let log = logres.with_context(|| anyhow!("Collecting logs for job on '{}'", ep.name()))?;
        let (paths, container_hash, script) = res.with_context(|| anyhow!("Running job on '{}'", ep.name()))?;

        dbmodels::Job::create(&self.db, &job_id, &self.submit, &endpoint, &package, &image, &container_hash, &script, &log)?;
        Ok(paths)
    }

}

struct LogReceiver<'a> {
    job_id: Uuid,
    log_receiver: UnboundedReceiver<LogItem>,
    bar: &'a ProgressBar,
    db: Arc<PgConnection>,
}

impl<'a> LogReceiver<'a> {
    async fn join(mut self) -> Result<String> {
        use resiter::Map;

        let mut success = None;
        let mut accu    = vec![];

        while let Some(logitem) = self.log_receiver.recv().await {
            match logitem {
                LogItem::Line(ref l) => {
                    // ignore
                },
                LogItem::Progress(u) => {
                    self.bar.set_position(u as u64);
                },
                LogItem::CurrentPhase(ref phasename) => {
                    self.bar.set_message(&format!("{} Phase: {}", self.job_id, phasename));
                },
                LogItem::State(Ok(ref s)) => {
                    self.bar.set_message(&format!("{} State Ok: {}", self.job_id, s));
                    success = Some(true);
                },
                LogItem::State(Err(ref e)) => {
                    self.bar.set_message(&format!("{} State Err: {}", self.job_id, e));
                    success = Some(false);
                },
            }
            accu.push(logitem);
        }

        match success {
            Some(true) => self.bar.finish_with_message(&format!("{} finished successfully", self.job_id)),
            Some(false) => self.bar.finish_with_message(&format!("{} finished with error", self.job_id)),
            None => self.bar.finish_with_message(&format!("{} finished", self.job_id)),
        }

        Ok({
            accu.into_iter()
                .map(|ll| ll.display())
                .map_ok(|d| d.to_string())
                .collect::<Result<Vec<String>>>()?
                .join("\n")
        })
    }
}

