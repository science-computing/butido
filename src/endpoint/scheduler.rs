use std::path::PathBuf;
use std::result::Result as RResult;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use diesel::PgConnection;
use futures::FutureExt;
use indicatif::ProgressBar;
use itertools::Itertools;
use tokio::stream::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedReceiver;
use uuid::Uuid;

use crate::db::models as dbmodels;
use crate::endpoint::Endpoint;
use crate::endpoint::EndpointConfiguration;
use crate::filestore::StagingStore;
use crate::job::JobResource;
use crate::job::RunnableJob;
use crate::log::LogItem;
use crate::util::progress::ProgressBars;
use crate::endpoint::ContainerError;

pub struct EndpointScheduler {
    log_dir: Option<PathBuf>,
    endpoints: Vec<Arc<RwLock<Endpoint>>>,

    staging_store: Arc<RwLock<StagingStore>>,
    db: Arc<PgConnection>,
    progressbars: ProgressBars,
    submit: crate::db::models::Submit,
}

impl EndpointScheduler {

    pub async fn setup(endpoints: Vec<EndpointConfiguration>, staging_store: Arc<RwLock<StagingStore>>, db: Arc<PgConnection>, progressbars: ProgressBars, submit: crate::db::models::Submit, log_dir: Option<PathBuf>) -> Result<Self> {
        let endpoints = Self::setup_endpoints(endpoints).await?;

        Ok(EndpointScheduler {
            log_dir,
            endpoints,
            staging_store,
            db,
            progressbars,
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
    pub async fn schedule_job(&self, job: RunnableJob, multibar: Arc<indicatif::MultiProgress>) -> Result<JobHandle> {
        let endpoint = self.select_free_endpoint().await?;

        Ok(JobHandle {
            log_dir: self.log_dir.clone(),
            bar: multibar.add(self.progressbars.job_bar(job.uuid())),
            endpoint,
            job,
            staging_store: self.staging_store.clone(),
            db: self.db.clone(),
            submit: self.submit.clone(),
        })
    }

    async fn select_free_endpoint(&self) -> Result<Arc<RwLock<Endpoint>>> {
        loop {
            let ep = self.endpoints
                .iter()
                .cloned()
                .map(|ep| async move {
                    ep.write()
                        .await
                        .number_of_running_containers()
                        .await
                        .map(|num_running| (num_running, ep.clone()))
                })
                .collect::<futures::stream::FuturesUnordered<_>>()
                .collect::<Result<Vec<_>>>()
                .await?
                .iter()
                .sorted_by(|tpla, tplb| tpla.0.cmp(&tplb.0))
                .map(|tpl| tpl.1.clone())
                .next();

            if let Some(endpoint) = ep {
                return Ok(endpoint)
            } else {
                trace!("No free endpoint found, retry...");
            }
        }
    }

}

pub struct JobHandle {
    log_dir: Option<PathBuf>,
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
    pub async fn run(self) -> RResult<Vec<dbmodels::Artifact>, ContainerError> {
        use crate::db::models as dbmodels;
        let (log_sender, log_receiver) = tokio::sync::mpsc::unbounded_channel::<LogItem>();
        let ep       = self.endpoint.read().await;
        let endpoint = dbmodels::Endpoint::create_or_fetch(&self.db, ep.name())?;
        let package  = dbmodels::Package::create_or_fetch(&self.db, self.job.package())?;
        let image    = dbmodels::Image::create_or_fetch(&self.db, self.job.image())?;
        let envs     = self.create_env_in_db()?;
        let job_id   = self.job.uuid().clone();
        trace!("Running on Job {} on Endpoint {}", job_id, ep.name());
        let res = ep
            .run_job(self.job, log_sender, self.staging_store);

        let logres = LogReceiver {
            log_dir: self.log_dir.as_ref(),
            job_id,
            log_receiver,
            bar: &self.bar,
        }.join();

        let (res, logres) = tokio::join!(res, logres);

        trace!("Found result for job {}: {:?}", job_id, res);
        let log = logres.with_context(|| anyhow!("Collecting logs for job on '{}'", ep.name()))?;
        let (paths, container_hash, script) = res.with_context(|| anyhow!("Running job on '{}'", ep.name()))?;

        let job = dbmodels::Job::create(&self.db, &job_id, &self.submit, &endpoint, &package, &image, &container_hash, &script, &log)?;
        for env in envs {
            let _ = dbmodels::JobEnv::create(&self.db, &job, &env)?;
        }

        // Have to do it the ugly way here because of borrowing semantics
        let mut r = vec![];
        for p in paths.iter() {
            r.push(dbmodels::Artifact::create(&self.db, p, false, &job)?);
        }
        Ok(r)
    }

    fn create_env_in_db(&self) -> Result<Vec<dbmodels::EnvVar>> {
        trace!("Creating environment in database");
        trace!("Hardcoded = {:?}", self.job.package().environment());
        trace!("Dynamic   = {:?}", self.job.resources());
        self.job
            .package()
            .environment()
            .as_ref()
            .map(|hm| {
                hm.iter()
                    .inspect(|(k, v)| trace!("Creating environment variable in database: {} = {}", k, v))
                    .map(|(k, v)| dbmodels::EnvVar::create_or_fetch(&self.db, k, v))
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default()
            .into_iter()
            .map(Ok)
            .chain({
                self.job
                    .resources()
                    .iter()
                    .filter_map(JobResource::env)
                    .inspect(|(k, v)| trace!("Creating environment variable in database: {} = {}", k, v))
                    .map(|(k, v)| dbmodels::EnvVar::create_or_fetch(&self.db, k, v))
            })
            .collect()
    }

}

struct LogReceiver<'a> {
    log_dir: Option<&'a PathBuf>,
    job_id: Uuid,
    log_receiver: UnboundedReceiver<LogItem>,
    bar: &'a ProgressBar,
}

impl<'a> LogReceiver<'a> {

    async fn join(mut self) -> Result<String> {
        use resiter::Map;

        let mut success = None;
        let mut accu    = vec![];
        let mut logfile = self.get_logfile()
            .await
            .transpose()?;

        while let Some(logitem) = self.log_receiver.recv().await {
            if let Some(lf) = logfile.as_mut() {
                lf.write_all(logitem.display()?.to_string().as_bytes()).await?;
                lf.write_all("\n".as_bytes()).await?;
            }

            match logitem {
                LogItem::Line(_) => {
                    // ignore
                },
                LogItem::Progress(u) => {
                    trace!("Setting bar to {}", u as u64);
                    self.bar.set_position(u as u64);
                    self.bar.set_message(&format!("Job: {} running...", self.job_id));
                },
                LogItem::CurrentPhase(ref phasename) => {
                    trace!("Setting bar phase to {}", phasename);
                    self.bar.set_message(&format!("Job: {} Phase: {}", self.job_id, phasename));
                },
                LogItem::State(Ok(ref s)) => {
                    trace!("Setting bar state to Ok: {}", s);
                    self.bar.set_message(&format!("Job: {} State Ok: {}", self.job_id, s));
                    success = Some(true);
                },
                LogItem::State(Err(ref e)) => {
                    trace!("Setting bar state to Err: {}", e);
                    self.bar.set_message(&format!("Job: {} State Err: {}", self.job_id, e));
                    success = Some(false);
                },
            }
            accu.push(logitem);
        }

        trace!("Finishing bar = {:?}", success);
        let finish_msg = match success {
            Some(true)  => format!("Job: {} finished successfully", self.job_id),
            Some(false) => format!("Job: {} finished with error", self.job_id),
            None        => format!("Job: {} finished", self.job_id),
        };
        self.bar.finish_with_message(&finish_msg);

        drop(self.bar);
        if let Some(mut lf) = logfile {
            let _ = lf.flush().await?;
        }

        Ok({
            accu.iter()
                .map(crate::log::LogItem::display)
                .map_ok(|d| d.to_string())
                .collect::<Result<Vec<String>>>()?
                .join("\n")
        })
    }

    async fn get_logfile(&self) -> Option<Result<tokio::io::BufWriter<tokio::fs::File>>> {
        if let Some(log_dir) = self.log_dir.as_ref() {
            Some({
                let path = log_dir.join(self.job_id.to_string()).join(".log");
                tokio::fs::OpenOptions::new()
                    .create(true)
                    .create_new(true)
                    .write(true)
                    .open(path)
                    .await
                    .map(tokio::io::BufWriter::new)
                    .map_err(Error::from)
            })
        } else {
            None
        }
    }

}

