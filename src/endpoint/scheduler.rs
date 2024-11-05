//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use colored::Colorize;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use diesel::PgConnection;
use getset::{CopyGetters, Getters};
use indicatif::ProgressBar;
use itertools::Itertools;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::RwLock;
use tracing::{debug, error, trace};
use uuid::Uuid;

use crate::db::models as dbmodels;
use crate::endpoint::Endpoint;
use crate::endpoint::EndpointConfiguration;
use crate::endpoint::EndpointHandle;
use crate::filestore::ArtifactPath;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::job::JobResource;
use crate::job::RunnableJob;
use crate::log::LogItem;

#[derive(Getters, CopyGetters)]
pub struct EndpointScheduler {
    log_dir: Option<PathBuf>,
    endpoints: Vec<Arc<Endpoint>>,
    #[getset(get = "pub")]
    max_endpoint_name_length: usize,

    staging_store: Arc<RwLock<StagingStore>>,
    release_stores: Vec<Arc<ReleaseStore>>,
    db: Pool<ConnectionManager<PgConnection>>,
    submit: crate::db::models::Submit,
}

impl EndpointScheduler {
    pub async fn setup(
        endpoints: Vec<EndpointConfiguration>,
        staging_store: Arc<RwLock<StagingStore>>,
        release_stores: Vec<Arc<ReleaseStore>>,
        db: Pool<ConnectionManager<PgConnection>>,
        submit: crate::db::models::Submit,
        log_dir: Option<PathBuf>,
    ) -> Result<Self> {
        let endpoints = crate::endpoint::util::setup_endpoints(endpoints).await?;
        let max_endpoint_name_length = endpoints
            .iter()
            .map(|ep| ep.name().len())
            .max()
            .unwrap_or(0);

        Ok(EndpointScheduler {
            log_dir,
            endpoints,
            max_endpoint_name_length,
            staging_store,
            release_stores,
            db,
            submit,
        })
    }

    /// Schedule a Job
    ///
    /// # Warning
    ///
    /// This function blocks as long as there is no free endpoint available!
    pub async fn schedule_job(
        &self,
        job: RunnableJob,
        bar: indicatif::ProgressBar,
    ) -> Result<JobHandle> {
        let endpoint = self.select_free_endpoint().await?;

        Ok(JobHandle {
            log_dir: self.log_dir.clone(),
            bar,
            endpoint,
            container_id: None,
            max_endpoint_name_length: self.max_endpoint_name_length,
            job,
            staging_store: self.staging_store.clone(),
            release_stores: self.release_stores.clone(),
            db: self.db.clone(),
            submit: self.submit.clone(),
        })
    }

    async fn select_free_endpoint(&self) -> Result<EndpointHandle> {
        loop {
            let ep = self
                .endpoints
                .iter()
                .filter(|ep| {
                    // filter out all running containers where the number of max jobs is reached
                    let r = ep.running_jobs() < ep.num_max_jobs();
                    trace!(
                        "Endpoint {} considered for scheduling job: {}",
                        ep.name(),
                        r
                    );
                    r
                })
                .sorted_by(|ep1, ep2| {
                    ep1.utilization()
                        .partial_cmp(&ep2.utilization())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .next();

            if let Some(endpoint) = ep {
                return Ok(EndpointHandle::new(endpoint.clone()));
            } else {
                trace!("No free endpoint found, retry...");
                tokio::task::yield_now().await
            }
        }
    }
}

#[derive(Clone)]
pub struct JobHandle {
    log_dir: Option<PathBuf>,
    endpoint: EndpointHandle,
    container_id: Option<String>,
    max_endpoint_name_length: usize,
    job: RunnableJob,
    bar: ProgressBar,
    db: Pool<ConnectionManager<PgConnection>>,
    staging_store: Arc<RwLock<StagingStore>>,
    release_stores: Vec<Arc<ReleaseStore>>,
    submit: crate::db::models::Submit,
}

impl std::fmt::Debug for JobHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "JobHandle ( job: {} )", self.job.uuid())
    }
}

impl JobHandle {
    pub async fn run(mut self) -> Result<Result<Vec<ArtifactPath>>> {
        let (log_sender, log_receiver) = tokio::sync::mpsc::unbounded_channel::<LogItem>();
        let endpoint_uri = self.endpoint.uri().clone();
        let endpoint_name = self.endpoint.name().clone();
        let endpoint =
            dbmodels::Endpoint::create_or_fetch(&mut self.db.get().unwrap(), self.endpoint.name())?;
        let package =
            dbmodels::Package::create_or_fetch(&mut self.db.get().unwrap(), self.job.package())?;
        let image =
            dbmodels::Image::create_or_fetch(&mut self.db.get().unwrap(), self.job.image())?;
        let envs = self.create_env_in_db()?;
        let job_id = *self.job.uuid();
        trace!(
            "Running on Job {} on Endpoint {}",
            job_id,
            self.endpoint.name()
        );
        let prepared_container = self
            .endpoint
            .prepare_container(
                &self.job,
                self.staging_store.clone(),
                self.release_stores.clone(),
            )
            .await?;
        let container_id = prepared_container.create_info().id.clone();
        self.container_id = Some(container_id.clone());
        let running_container = prepared_container
            .start()
            .await
            .with_context(|| {
                Self::create_job_run_error(
                    &job_id,
                    &package.name,
                    &package.version,
                    &endpoint_uri,
                    &container_id,
                )
            })?
            .execute_script(log_sender);

        let logres = LogReceiver {
            endpoint_name: endpoint_name.as_ref(),
            max_endpoint_name_length: &self.max_endpoint_name_length,
            container_id_chrs: container_id.chars().take(7).collect(),
            package_name: &package.name,
            package_version: &package.version,
            log_dir: self.log_dir.as_ref(),
            job: self.job.clone(),
            log_receiver,
            bar: self.bar.clone(),
        }
        .join();
        drop(self.bar.clone());

        let (run_container, logres) = tokio::join!(running_container, logres);
        let log =
            logres.with_context(|| anyhow!("Collecting logs for job on '{}'", endpoint_name))?;
        let run_container = run_container
            .with_context(|| anyhow!("Running container {} failed", container_id))
            .with_context(|| {
                Self::create_job_run_error(
                    &job_id,
                    &package.name,
                    &package.version,
                    &endpoint_uri,
                    &container_id,
                )
            })?;

        let job = dbmodels::Job::create(
            &mut self.db.get().unwrap(),
            &job_id,
            &self.submit,
            &endpoint,
            &package,
            &image,
            &run_container.container_hash(),
            run_container.script(),
            &log,
        )
        .context("Recording job that is ready in database")?;

        trace!("DB: Job entry for job {} created: {}", job.uuid, job.id);
        for env in envs {
            dbmodels::JobEnv::create(&mut self.db.get().unwrap(), &job, &env).with_context(
                || {
                    format!(
                        "Creating Environment Variable mapping for Job: {}",
                        job.uuid
                    )
                },
            )?;
        }

        let res: crate::endpoint::FinalizedContainer = run_container
            .finalize(self.staging_store.clone())
            .await
            .context("Finalizing container")
            .with_context(|| {
                Self::create_job_run_error(
                    &job.uuid,
                    &package.name,
                    &package.version,
                    &endpoint_uri,
                    &container_id,
                )
            })?;

        trace!("Found result for job {}: {:?}", job_id, res);
        let (paths, res) = res.unpack();
        let res = res
            .with_context(|| anyhow!("Error during running job on '{}'", endpoint_name))
            .with_context(|| {
                Self::create_job_run_error(
                    &job.uuid,
                    &package.name,
                    &package.version,
                    &endpoint_uri,
                    &container_id,
                )
            })
            .map_err(Error::from);

        if res.is_err() {
            trace!("Error was returned from script");
            return Ok({
                res.map(|_| vec![]) // to have the proper type, will never be executed
            });
        }

        // Have to do it the ugly way here because of borrowing semantics
        let mut r = vec![];
        let staging_read = self.staging_store.read().await;
        for p in paths.iter() {
            trace!("DB: Creating artifact entry for path: {}", p.display());
            let _ = dbmodels::Artifact::create(&mut self.db.get().unwrap(), p, &job)?;
            r.push({
                staging_read
                    .get(p)
                    .ok_or_else(|| anyhow!("Artifact not in store: {:?}", p))?
                    .clone()
            });
        }
        Ok(Ok(r))
    }

    /// Helper to create an error object with a nice message.
    fn create_job_run_error(
        job_id: &Uuid,
        package_name: &str,
        package_version: &str,
        endpoint_uri: &str,
        container_id: &str,
    ) -> Error {
        anyhow!(indoc::formatdoc!(
            r#"Error while running job for {package_name} {package_version} with id:

            {job_id}

        To debug, connect to Docker using:

            {docker_connect_string}

        or, to use butido to show the log of the job, run:

            butido db log-of {job_id} | less -SR +G
        "#,
            job_id = job_id.to_string().red(),
            package_name = package_name.to_string().red(),
            package_version = package_version.to_string().red(),
            docker_connect_string =
                format!("docker --host {endpoint_uri} exec -it {container_id} /bin/bash")
                    .yellow()
                    .bold(),
        ))
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
                    .inspect(|(k, v)| {
                        trace!("Creating environment variable in database: {} = {}", k, v)
                    })
                    .map(|(k, v)| {
                        dbmodels::EnvVar::create_or_fetch(&mut self.db.get().unwrap(), k, v)
                    })
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
                    .inspect(|(k, v)| {
                        trace!("Creating environment variable in database: {} = {}", k, v)
                    })
                    .map(|(k, v)| {
                        dbmodels::EnvVar::create_or_fetch(&mut self.db.get().unwrap(), k, v)
                    })
            })
            .collect()
    }
}

impl Drop for JobHandle {
    fn drop(&mut self) {
        debug!("Cleaning up JobHandle");
        if self.container_id.is_some() {
            debug!("Container was already started");
            let docker = self.endpoint.docker().clone();
            let container_id = self.container_id.take().unwrap();

            tokio::spawn(async move {
                let container = docker.containers().get(&container_id);
                let container_info = container.inspect().await.unwrap();

                if container_info.state.running {
                    debug!("Container is still running, cleaning up...");
                    match container.kill(None).await {
                        Ok(_) => debug!("Stopped container with id: {}", container_id),
                        Err(e) => {
                            error!("Failed to stop container with id: {}\n{}", container_id, e)
                        }
                    }
                } else {
                    debug!("Container has already finished");
                }
            });
        } else {
            debug!("No container created");
        }
    }
}

struct LogReceiver<'a> {
    endpoint_name: &'a str,
    max_endpoint_name_length: &'a usize,
    container_id_chrs: String,
    package_name: &'a str,
    package_version: &'a str,
    log_dir: Option<&'a PathBuf>,
    job: RunnableJob,
    log_receiver: UnboundedReceiver<LogItem>,
    bar: ProgressBar,
}

impl<'a> LogReceiver<'a> {
    async fn join(mut self) -> Result<String> {
        let mut success = None;
        // Reserve a reasonable amount of elements.
        let mut accu = Vec::with_capacity(4096);

        let mut logfile = self
            .get_logfile()
            .await
            .transpose()
            .context("Getting Logfile")?;

        // The timeout for the log-receive-timeout
        //
        // We're using a rather small timeout of just 250ms here, because we have some worktime
        // overhead as well, and we want to ping the progressbar for the process in a way so that
        // it updates each second.
        // Having a timeout of 1 sec proofed to be too long to update the time field in the
        // progress bar secondly.
        let timeout_duration = std::time::Duration::from_millis(250);
        let max_endpoint_name_length = self.max_endpoint_name_length;

        loop {
            // Timeout for receiving from the log receiver channel
            // This way we can update (`tick()`) the progress bar and show the user that things are
            // happening, even if there was no log output for several seconds.
            let logitem =
                match tokio::time::timeout(timeout_duration, self.log_receiver.recv()).await {
                    Err(_ /* elapsed */) => {
                        self.bar.tick(); // just ping the progressbar here
                        continue;
                    }

                    Ok(None) => break, // if the log is empty, we're done
                    Ok(Some(logitem)) => logitem,
                };

            if let Some(lf) = logfile.as_mut() {
                lf.write_all(logitem.display()?.to_string().as_bytes())
                    .await?;
                lf.write_all(b"\n").await?;
            }

            match logitem {
                LogItem::Line(_) => {
                    // ignore
                }
                LogItem::Progress(u) => {
                    trace!("Setting bar to {}", u as u64);
                    self.bar.set_position(u as u64);
                }
                LogItem::CurrentPhase(ref phasename) => {
                    trace!("Setting bar phase to {}", phasename);
                    self.bar.set_message(format!(
                        "{:<max_endpoint_name_length$} {} {} {} {} {} {}",
                        self.endpoint_name,
                        self.container_id_chrs,
                        self.job.uuid(),
                        "\u{2588}\u{2588}".yellow(),
                        self.package_name,
                        self.package_version,
                        phasename
                    ));
                }
                LogItem::State(Ok(())) => {
                    trace!("Setting bar state to Ok");
                    self.bar.set_message(format!(
                        "{:<max_endpoint_name_length$} {} {} {} {} {}",
                        self.endpoint_name,
                        self.container_id_chrs,
                        self.job.uuid(),
                        "\u{2588}\u{2588}".green(),
                        self.package_name,
                        self.package_version
                    ));
                    success = Some(true);
                }
                LogItem::State(Err(ref e)) => {
                    trace!("Setting bar state to Err: {}", e);
                    self.bar.set_message(format!(
                        "{:<max_endpoint_name_length$} {} {} {} {} {} {}",
                        self.endpoint_name,
                        self.container_id_chrs,
                        self.job.uuid(),
                        "\u{2588}\u{2588}".red(),
                        self.package_name,
                        self.package_version,
                        e
                    ));
                    success = Some(false);
                }
            }
            accu.push(logitem);
        }

        trace!("Finishing bar = {:?}", success);
        let finish_msg = match success {
            Some(true) => "\u{2588}\u{2588}".green(),
            Some(false) => "\u{2588}\u{2588}".red(),
            None => "\u{2588}\u{2588}".blue(),
        };

        let finish_msg = format!(
            "{:<max_endpoint_name_length$} {} {} {} {} {}",
            self.endpoint_name,
            self.container_id_chrs,
            self.job.uuid(),
            finish_msg,
            self.package_name,
            self.package_version,
        );
        self.bar.finish_with_message(finish_msg);

        if let Some(mut lf) = logfile {
            lf.flush().await?;
        }

        Ok({
            accu.iter()
                .map(crate::log::LogItem::raw)
                .collect::<Result<Vec<String>>>()?
                .join("\n")
        })
    }

    async fn get_logfile(&self) -> Option<Result<tokio::io::BufWriter<tokio::fs::File>>> {
        if let Some(log_dir) = self.log_dir.as_ref() {
            Some({
                let path = log_dir.join(format!(
                    "{}-{}-{}-{}.log",
                    self.package_name,
                    self.package_version,
                    self.job.image(),
                    self.job.uuid()
                ));
                tokio::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&path)
                    .await
                    .map(tokio::io::BufWriter::new)
                    .with_context(|| anyhow!("Opening {}", path.display()))
                    .map_err(Error::from)
            })
        } else {
            None
        }
    }
}
