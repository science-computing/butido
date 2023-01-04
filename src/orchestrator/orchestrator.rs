//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

#![allow(unused)]

use std::borrow::Borrow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Error;
use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use diesel::PgConnection;
use git2::Repository;
use indicatif::ProgressBar;
use itertools::Itertools;
use tracing::{debug, trace, error};
use resiter::FilterMap;
use tokio::sync::RwLock;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio_stream::StreamExt;
use typed_builder::TypedBuilder;
use uuid::Uuid;

use crate::config::Configuration;
use crate::db::models as dbmodels;
use crate::endpoint::EndpointConfiguration;
use crate::endpoint::EndpointScheduler;
use crate::filestore::ArtifactPath;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::job::Dag;
use crate::job::JobDefinition;
use crate::job::RunnableJob;
use crate::orchestrator::util::*;
use crate::source::SourceCache;
use crate::util::EnvironmentVariableName;
use crate::util::progress::ProgressBars;

#[cfg_attr(doc, aquamarine::aquamarine)]
/// The Orchestrator
///
/// The Orchestrator is used to orchestrate the work on one submit.
/// On a very high level: It uses a [Dag](crate::job::Dag) to build a number (list) of
/// [JobTasks](crate::orchestrator::JobTask) that is then run concurrently.
///
/// Because of the implementation of [JobTask], the work happens in
/// form of a tree, propagating results to the root (which is held by the Orchestrator itself).
/// The Orchestrator also holds the connection to the database, the access to the filesystem via
/// the [ReleaseStore](crate::filestore::ReleaseStore) and the
/// [StagingStore](crate::filestore::StagingStore), which are merged into a
/// [MergedStores](crate::filestore::MergedStores) object.
///
///
/// # Control Flow
///
/// This section describes the control flow starting with the construction of the Orchestrator
/// until the exit of the Orchestrator.
///
/// ```mermaid
/// sequenceDiagram
///     participant Caller as User
///     participant O   as Orchestrator
///     participant JT1 as JobTask
///     participant JT2 as JobTask
///     participant SCH as Scheduler
///     participant EP1 as Endpoint
///
///     Caller->>+O: run()
///         O->>+O: run_tree()
///
///             par Starting jobs
///                 O->>+JT1: run()
///             and
///                 O->>+JT2: run()
///             end
///
///             par Working on jobs
///                 loop until dependencies received
///                     JT1->>JT1: recv()
///                 end
///
///                 JT1->>+JT1: build()
///                 JT1->>SCH: schedule(job)
///                 SCH->>+EP1: run(job)
///                 EP1->>-SCH: [Artifacts]
///                 SCH->>JT1: [Artifacts]
///                 JT1->>-JT1: send_artifacts
///             and
///                 loop until dependencies received
///                     JT2->>JT2: recv()
///                 end
///
///                 JT2->>+JT2: build()
///                 JT2->>SCH: schedule(job)
///                 SCH->>+EP1: run(job)
///                 EP1->>-SCH: [Artifacts]
///                 SCH->>JT2: [Artifacts]
///                 JT2->>-JT2: send_artifacts
///             end
///
///         O->>-O: recv(): [Artifacts]
///     O-->>-Caller: [Artifacts]
/// ```
///
/// Because the chart from above is already rather big, the described submit works with only two
/// packages being built on one endpoint.
///
/// The Orchestrator starts the JobTasks in parallel, and they are executed in parallel.
/// Each JobTask receives dependencies until there are no more dependencies to receive. Then, it
/// starts building the job by forwarding the actual job to the scheduler, which in turn schedules
/// the Job on one of the endpoints.
///
///
/// # JobTask
///
/// A [JobTask] is run in parallel to all other JobTasks (concurrently on the tokio runtime).
/// Leveraging the async runtime, it waits until it received all dependencies from it's "child
/// tasks" (the nodes further down in the tree of jobs), which semantically means that it blocks
/// until it can run.
///
/// ```mermaid
/// graph TD
///     r[Receiving deps]
///     dr{All deps received}
///     ae{Any error received}
///     se[Send errors to parent]
///     b[Schedule job]
///     be{error during sched}
///     asum[received artifacts + artifacts from sched]
///     sa[Send artifacts to parent]
///
///     r --> dr
///     dr -->|no| r
///     dr -->|yes| ae
///
///     ae -->|yes| se
///     ae -->|no| b
///     b --> be
///     be -->|yes| se
///     be -->|no| asum
///     asum --> sa
/// ```
///
/// The "root" JobTask sends its artifacts to the orchestrator, which returns them to the caller.
///
pub struct Orchestrator<'a> {
    scheduler: EndpointScheduler,
    progress_generator: ProgressBars,
    staging_store: Arc<RwLock<StagingStore>>,
    release_stores: Vec<Arc<ReleaseStore>>,
    source_cache: SourceCache,
    jobdag: Dag,
    config: &'a Configuration,
    repository: Repository,
    database: Arc<Mutex<PgConnection>>,
}

#[derive(TypedBuilder)]
pub struct OrchestratorSetup<'a> {
    progress_generator: ProgressBars,
    endpoint_config: Vec<EndpointConfiguration>,
    staging_store: Arc<RwLock<StagingStore>>,
    release_stores: Vec<Arc<ReleaseStore>>,
    source_cache: SourceCache,
    jobdag: Dag,
    database: Arc<Mutex<PgConnection>>,
    submit: dbmodels::Submit,
    log_dir: Option<PathBuf>,
    config: &'a Configuration,
    repository: Repository,
}

impl<'a> OrchestratorSetup<'a> {
    pub async fn setup(self) -> Result<Orchestrator<'a>> {
        let scheduler = EndpointScheduler::setup(
            self.endpoint_config,
            self.staging_store.clone(),
            self.release_stores.clone(),
            self.database.clone(),
            self.submit.clone(),
            self.log_dir,
        )
        .await?;

        Ok(Orchestrator {
            scheduler,
            staging_store: self.staging_store.clone(),
            release_stores: self.release_stores.clone(),
            progress_generator: self.progress_generator,
            source_cache: self.source_cache,
            jobdag: self.jobdag,
            config: self.config,
            database: self.database,
            repository: self.repository,
        })
    }
}

/// Helper type
///
/// Represents a result that came from the run of a job inside a container
///
/// It is either a list of artifacts with the UUID of the job they were produced by,
/// or a UUID and an Error object, where the UUID is the job UUID and the error is the
/// anyhow::Error that was issued.
///
/// The artifacts are encapsulated into a `ProducedArtifact`, see the documentation of the type for
/// why.
type JobResult = std::result::Result<HashMap<Uuid, Vec<ProducedArtifact>>, HashMap<Uuid, Error>>;

/// A type that represents whether an artifact was built or reused from an old job
///
/// This is necessary to decide in dependent jobs whether a package needs to be rebuild even though
/// the script and environment did not change.
///
/// E.G.: If a libA depends on libB, if libB changed and needs to be rebuilt, we need to rebuilt
/// all packages that depend (directly or indirectly) on that library.
#[derive(Clone, Debug)]
enum ProducedArtifact {
    Built(ArtifactPath),
    Reused(ArtifactPath),
}

impl ProducedArtifact {
    /// Get whether the ProducedArtifact was built or reused from another job
    fn was_build(&self) -> bool {
        std::matches!(self, ProducedArtifact::Built(_))
    }

    /// Unpack the ProducedArtifact object into the ArtifactPath object it contains
    fn unpack(self) -> ArtifactPath {
        match self {
            ProducedArtifact::Built(a) => a,
            ProducedArtifact::Reused(a) => a,
        }
    }
}

impl Borrow<ArtifactPath> for ProducedArtifact {
    fn borrow(&self) -> &ArtifactPath  {
        match self {
            ProducedArtifact::Built(a) => a,
            ProducedArtifact::Reused(a) => a,
        }
    }
}

impl<'a> Orchestrator<'a> {
    pub async fn run(self, output: &mut Vec<ArtifactPath>) -> Result<HashMap<Uuid, Error>> {
        let (results, errors) = self.run_tree().await?;
        output.extend(results.into_iter());
        Ok(errors)
    }

    async fn run_tree(self) -> Result<(Vec<ArtifactPath>, HashMap<Uuid, Error>)> {
        let multibar = Arc::new({
            let mp = indicatif::MultiProgress::new();
            if self.progress_generator.hide() {
                mp.set_draw_target(indicatif::ProgressDrawTarget::hidden());
            }
            mp
        });

        let git_author_env = {
            self.config
                .containers()
                .git_author()
                .as_ref()
                .map(|varname| -> Result<_> {
                    let username = self.repository
                        .config()?
                        .get_string("user.name")?;

                    Ok((varname.clone(), username))
                })
                .transpose()?
        };

        let git_commit_env = {
            self.config
                .containers()
                .git_commit_hash()
                .as_ref()
                .map(|varname| -> Result<_> {
                    let hash = crate::util::git::get_repo_head_commit_hash(&self.repository)?;
                    Ok((varname.clone(), hash))
                })
                .transpose()?
        };

        // For each job in the jobdag, built a tuple with
        //
        // 1. The receiver that is used by the task to receive results from dependency tasks from
        // 2. The task itself (as a TaskPreparation object)
        // 3. The sender, that can be used to send results to this task
        // 4. An Option<Sender> that this tasks uses to send its results with
        //    This is an Option<> because we need to set it later and the root of the tree needs a
        //    special handling, as this very function will wait on a receiver that gets the results
        //    of the root task
        let jobs: Vec<(Receiver<JobResult>, TaskPreparation, Sender<JobResult>, _)> = self.jobdag
            .iter()
            .map(|jobdef| {
                // We initialize the channel with 100 elements here, as there is unlikely a task
                // that depends on 100 other tasks.
                // Either way, this might be increased in future.
                let (sender, receiver) = tokio::sync::mpsc::channel(100);

                trace!("Creating TaskPreparation object for job {}", jobdef.job.uuid());
                let bar = self.progress_generator.bar()?;
                let bar = multibar.add(bar);
                bar.set_length(100);
                let tp = TaskPreparation {
                    jobdef,

                    bar,
                    config: self.config,
                    git_author_env: git_author_env.as_ref(),
                    git_commit_env: git_commit_env.as_ref(),
                    source_cache: &self.source_cache,
                    scheduler: &self.scheduler,
                    staging_store: self.staging_store.clone(),
                    release_stores: self.release_stores.clone(),
                    database: self.database.clone(),
                };

                Ok((receiver, tp, sender, std::cell::RefCell::new(None as Option<Vec<Sender<JobResult>>>)))
            })
            .collect::<Result<Vec<_>>>()?;

        // Associate tasks with their appropriate sender
        //
        // Right now, the tuple yielded from above contains (rx, task, tx, _), where rx and tx belong
        // to eachother.
        // But what we need is the tx (sender) that the task should send its result to, of course.
        //
        // So this algorithm in plain text is:
        //   for each job
        //      find the job that depends on this job
        //      use the sender of the found job and set it as sender for this job
        for job in jobs.iter() {
            if let Some(mut v) = job.3.borrow_mut().as_mut() {
                v.extend({
                jobs.iter()
                    .filter(|j| j.1.jobdef.dependencies.contains(job.1.jobdef.job.uuid()))
                    .map(|j| j.2.clone())
                });

                continue;
            }

            // else, but not in else {} because of borrowing
            *job.3.borrow_mut() = {
                let depending_on_job = jobs.iter()
                    .filter(|j| j.1.jobdef.dependencies.contains(job.1.jobdef.job.uuid()))
                    .map(|j| {
                        if j.1.jobdef.job.uuid() == job.1.jobdef.job.uuid() {
                            Err(anyhow!("Package does depend on itself: {} {}",
                                    job.1.jobdef.job.package().name(),
                                    job.1.jobdef.job.package().version()))
                        } else {
                            Ok(j)
                        }
                    })
                    .map_ok(|j| j.2.clone())
                    .collect::<Result<Vec<Sender<JobResult>>>>()?;

                trace!("{:?} is depending on {}", depending_on_job, job.1.jobdef.job.uuid());
                if depending_on_job.is_empty() {
                    None
                } else {
                    Some(depending_on_job)
                }
            };
        }

        // Find the id of the root task
        //
        // By now, all tasks should be associated with their respective sender.
        // Only one has None sender: The task that is the "root" of the tree.
        // By that property, we can find the root task.
        //
        // Here, we copy its uuid, because we need it later.
        let root_job_id = jobs.iter()
            .find(|j| j.3.borrow().is_none())
            .map(|j| j.1.jobdef.job.uuid())
            .ok_or_else(|| anyhow!("Failed to find root task"))?;
        trace!("Root job id = {}", root_job_id);

        // Create a sender and a receiver for the root of the tree
        let (root_sender, mut root_receiver) = tokio::sync::mpsc::channel(100);

        // Make all prepared jobs into real jobs and run them
        //
        // This maps each TaskPreparation with its sender and receiver to a JobTask and calls the
        // async fn JobTask::run() to run the task.
        //
        // The JobTask::run implementation handles the rest, we just have to wait for all futures
        // to succeed.
        let running_jobs = jobs
            .into_iter()
            .map(|prep| {
                trace!("Creating JobTask for = {}", prep.1.jobdef.job.uuid());
                // the sender is set or we need to use the root sender
                let sender = prep.3.into_inner().unwrap_or_else(|| vec![root_sender.clone()]);
                JobTask::new(prep.0, prep.1, sender)
            })
            .inspect(|task| trace!("Running: {}", task.jobdef.job.uuid()))
            .map(|task| task.run())
            .collect::<futures::stream::FuturesUnordered<_>>();
        debug!("Built {} jobs", running_jobs.len());

        running_jobs.collect::<Result<()>>().await?;
        trace!("All jobs finished");
        match root_receiver.recv().await {
            None                     => Err(anyhow!("No result received...")),
            Some(Ok(results)) => {
                let results = results.into_iter()
                    .flat_map(|tpl| tpl.1.into_iter())
                    .map(ProducedArtifact::unpack)
                    .collect();
                Ok((results, HashMap::with_capacity(0)))
            },
            Some(Err(errors))        => Ok((vec![], errors)),
        }
    }
}

/// Helper type: A task with all things attached, but not sender and receivers
///
/// This is the preparation of the JobTask, but without the associated sender and receiver, because
/// it is not mapped to the task yet.
///
/// This simply holds data and does not contain any more functionality
struct TaskPreparation<'a> {
    jobdef: JobDefinition<'a>,

    bar: ProgressBar,

    config: &'a Configuration,
    git_author_env: Option<&'a (EnvironmentVariableName, String)>,
    git_commit_env: Option<&'a (EnvironmentVariableName, String)>,
    source_cache: &'a SourceCache,
    scheduler: &'a EndpointScheduler,
    staging_store: Arc<RwLock<StagingStore>>,
    release_stores: Vec<Arc<ReleaseStore>>,
    database: Arc<Mutex<PgConnection>>,
}

/// Helper type for executing one job task
///
/// This type represents a task for a job that can immediately be executed (see `JobTask::run()`).
struct JobTask<'a> {
    jobdef: JobDefinition<'a>,

    bar: ProgressBar,

    config: &'a Configuration,
    git_author_env: Option<&'a (EnvironmentVariableName, String)>,
    git_commit_env: Option<&'a (EnvironmentVariableName, String)>,
    source_cache: &'a SourceCache,
    scheduler: &'a EndpointScheduler,
    staging_store: Arc<RwLock<StagingStore>>,
    release_stores: Vec<Arc<ReleaseStore>>,
    database: Arc<Mutex<PgConnection>>,

    /// Channel where the dependencies arrive
    receiver: Receiver<JobResult>,

    /// Channel to send the own build outputs to
    sender: Vec<Sender<JobResult>>,
}


/// Implement Drop to close the progress bar
///
/// This implementation is a bit of a hack.
/// Because all `JobTask`s are `JobTask::run()` in parallel, but there is no IPC _between_ the
/// tasks (there is IPC between childs and parents, but not between all JobTask objects), we never
/// know whether any other task errored when the JobTask object is destructed.
///
/// One way to implement this would be to add multi-cast IPC between all `JobTask` objects, with some
/// BUS like structure where all `JobTask`s can send messages to and listen to.
/// But that's non-trivial and a lot of overhead, of course.
///
/// The trick here is, that the progressbar is either finished when `drop()` is called, which means
/// that the `JobTask` is dropped because it finished,
/// or the progressbar is not finished yet, which means that the `JobTask` is dropped because the
/// runtime stops running it because some other `JobTask` errored.
///
/// In the latter case, we cleanup by telling the progressbar to finish.
impl<'a> Drop for JobTask<'a> {
    fn drop(&mut self) {
        if !self.bar.is_finished() {
            // If there are dependencies, the error is probably from another task
            // If there are no dependencies, the error was caused by something else
            let errmsg = if self.jobdef.dependencies.is_empty() {
                "error occured"
            } else {
                "error on other task"
            };

            self.bar.finish_with_message(format!("[{} {} {}] Stopped, {msg}",
                self.jobdef.job.uuid(),
                self.jobdef.job.package().name(),
                self.jobdef.job.package().version(),
                msg = errmsg));
        }
    }
}

impl<'a> JobTask<'a> {
    fn new(receiver: Receiver<JobResult>, prep: TaskPreparation<'a>, sender: Vec<Sender<JobResult>>) -> Self {
        let bar = prep.bar.clone();
        bar.set_message(format!("[{} {} {}]: Booting",
            prep.jobdef.job.uuid(),
            prep.jobdef.job.package().name(),
            prep.jobdef.job.package().version()
        ));
        JobTask {
            jobdef: prep.jobdef,

            bar,

            config: prep.config,
            git_author_env: prep.git_author_env,
            git_commit_env: prep.git_commit_env,
            source_cache: prep.source_cache,
            scheduler: prep.scheduler,
            staging_store: prep.staging_store,
            release_stores: prep.release_stores,
            database: prep.database.clone(),

            receiver,
            sender,
        }
    }

    /// Run the job
    ///
    /// This function runs the job from this object on the scheduler as soon as all dependend jobs
    /// returned successfully.
    async fn run(mut self) -> Result<()> {
        debug!("[{}]: Running", self.jobdef.job.uuid());
        debug!("[{}]: Waiting for dependencies = {:?}", self.jobdef.job.uuid(), {
            self.jobdef.dependencies.iter().map(|u| u.to_string()).collect::<Vec<String>>()
        });

        let dep_len = self.jobdef.dependencies.len();
        // A list of job run results from dependencies that were received from the tasks for the
        // dependencies
        let mut received_dependencies: HashMap<Uuid, Vec<ProducedArtifact>> = HashMap::with_capacity(dep_len);

        // A list of errors that were received from the tasks for the dependencies
        let mut received_errors: HashMap<Uuid, Error> = HashMap::with_capacity(dep_len);

        // Helper function to check whether all UUIDs are in a list of UUIDs
        let all_dependencies_are_in = |dependency_uuids: &[Uuid], list: &HashMap<Uuid, Vec<_>>| {
            dependency_uuids.iter().all(|dependency_uuid| {
                list.keys().any(|id| id == dependency_uuid)
            })
        };

        // as long as the job definition lists dependencies that are not in the received_dependencies list...
        while !all_dependencies_are_in(&self.jobdef.dependencies, &received_dependencies) {
            // Update the status bar message
            self.bar.set_message({
                format!("[{} {} {}]: Waiting ({}/{})...",
                    self.jobdef.job.uuid(),
                    self.jobdef.job.package().name(),
                    self.jobdef.job.package().version(),
                    received_dependencies.iter().filter(|(rd_uuid, _)| self.jobdef.dependencies.contains(rd_uuid)).count(),
                    dep_len)
            });
            trace!("[{}]: Updated bar", self.jobdef.job.uuid());

            trace!("[{}]: receiving...", self.jobdef.job.uuid());
            // receive from the receiver
            let continue_receiving = self.perform_receive(&mut received_dependencies, &mut received_errors).await?;

            trace!("[{}]: Received errors = {}", self.jobdef.job.uuid(), received_errors.display_error_map());
            // if there are any errors from child tasks
            if !received_errors.is_empty() {
                // send them to the parent,...
                //
                // We only send to one parent, because it doesn't matter
                // And we know that we have at least one sender
                error!("[{}]: Received errors = {}", self.jobdef.job.uuid(), received_errors.display_error_map());
                self.sender[0].send(Err(received_errors)).await;

                // ... and stop operation, because the whole tree will fail anyways.
                self.bar.finish_with_message(format!("[{} {} {}] Stopping, errors from child received",
                    self.jobdef.job.uuid(),
                    self.jobdef.job.package().name(),
                    self.jobdef.job.package().version()));
                return Ok(())
            }

            if !continue_receiving {
                break;
            }
        }

        // Check if any of the received dependencies was built (and not reused).
        // If any dependency was built, we need to build as well.
        let any_dependency_was_built = received_dependencies.values()
            .flat_map(|v| v.iter())
            .any(ProducedArtifact::was_build);

        // If no dependency was built, we can check for replacements for this job as well, so
        // check if a job that looks very similar to this job has already produced artifacts.
        // If it has, simply return those (plus the received ones)
        if !any_dependency_was_built {
            let staging_store = self.staging_store.read().await;

            // Use the environment of the job definition, as it appears in the job DAG.
            //
            // This is because we do not have access to the commandline-passed (additional)
            // environment variables at this point. But using the JobResource::env() variables
            // works as well.
            let additional_env = self.jobdef.job.resources()
                .iter()
                .filter_map(crate::job::JobResource::env)
                .map(|(k, v)| (k.clone(), v.clone()))
                .chain(self.git_author_env.cloned().into_iter())
                .chain(self.git_commit_env.cloned().into_iter())
                .collect::<Vec<_>>();

            let replacement_artifacts = crate::db::FindArtifacts::builder()
                .database_connection(self.database.clone())
                .config(self.config)
                .package(self.jobdef.job.package())
                .release_stores(&self.release_stores)
                .image_name(Some(self.jobdef.job.image()))

                // We can simply pass the staging store here, because it doesn't hurt. There are
                // two scenarios:
                //
                // 1. We are in a fresh build for a package. In this case, the artifacts for this
                //    very build are not in there yet, and there won't be any artifacts from the
                //    staging store (possibly from the release store, which would be fine).
                // 2. We are in a re-build, where the user passed the staging store to the build
                //    subcommand. In this case, there might be an artifact for this job in the
                //    staging store. In this case, we want to use it as a replacement, of course.
                //
                // The fact that released artifacts are returned prefferably from this function
                // call does not change anything, because if there is an artifact that's a released
                // one that matches this job, we should use it anyways.
                .staging_store(Some(&staging_store))
                .env_filter(&additional_env)
                .script_filter(true)
                .build()
                .run()?;

            debug!("[{}]: Found {} replacement artifacts", self.jobdef.job.uuid(), replacement_artifacts.len());
            trace!("[{}]: Found replacement artifacts: {:?}", self.jobdef.job.uuid(), replacement_artifacts);
            let mut artifacts = replacement_artifacts
                .into_iter()

                // First of all, we sort by whether the artifact path is in the staging store,
                // because we prefer staging store artifacts at this point.
                .sorted_by(|(p1, _), (p2, _)| {
                    let r1 = p1.is_in_staging_store(&staging_store);
                    let r2 = p2.is_in_staging_store(&staging_store);
                    r1.cmp(&r2)
                })

                // We don't need duplicates here, so remove them by making the iterator unique
                // If we have two artifacts that are the same, the one in the staging store will be
                // preffered in the next step
                .unique_by(|tpl| tpl.0.artifact_path().clone())

                // Fetch the artifact from the staging store, if there is one.
                // If there is none, try the release store.
                // If there is none, there won't be a replacement artifact
                .filter_map(|(full_artifact_path, _)| {
                    trace!("Searching for {:?} in stores", full_artifact_path.display());
                    if let Some(ap) = staging_store.get(full_artifact_path.artifact_path()) {
                        Some(ap.clone())
                    } else {
                        self.release_stores
                            .iter()
                            .find_map(|rs| rs.get(full_artifact_path.artifact_path()))
                            .cloned()
                    }
                })
                .map(ProducedArtifact::Reused)
                .collect::<Vec<ProducedArtifact>>();

            if !artifacts.is_empty() {
                received_dependencies.insert(*self.jobdef.job.uuid(), artifacts);
                trace!("[{}]: Sending to parent: {:?}", self.jobdef.job.uuid(), received_dependencies);
                for s in self.sender.iter() {
                    s.send(Ok(received_dependencies.clone()))
                        .await
                        .context("Cannot send received dependencies to parent")
                        .with_context(|| {
                            format!("Sending-Channel is closed in Task for {}: {} {}",
                                self.jobdef.job.uuid(),
                                self.jobdef.job.package().name(),
                                self.jobdef.job.package().version())
                        })?;
                }
                self.bar.finish_with_message(format!("[{} {} {}] Reusing artifact",
                    self.jobdef.job.uuid(),
                    self.jobdef.job.package().name(),
                    self.jobdef.job.package().version()));
                return Ok(())
            }
        }

        // Map the list of received dependencies from
        //      Vec<(Uuid, Vec<ArtifactPath>)>
        // to
        //      Vec<ArtifactPath>
        let dependency_artifacts = received_dependencies
            .values()
            .flat_map(|v| v.iter())
            .map(ProducedArtifact::borrow)
            .cloned()
            .collect::<Vec<ArtifactPath>>();
        trace!("[{}]: Dependency artifacts = {:?}", self.jobdef.job.uuid(), dependency_artifacts);
        self.bar.set_message(format!("[{} {} {}]: Preparing...",
            self.jobdef.job.uuid(),
            self.jobdef.job.package().name(),
            self.jobdef.job.package().version()
        ));

        // Create a RunnableJob object
        let runnable = RunnableJob::build_from_job(
            self.jobdef.job,
            self.source_cache,
            self.config,
            self.git_author_env,
            self.git_commit_env,
            dependency_artifacts)?;

        self.bar.set_message(format!("[{} {} {}]: Scheduling...",
            self.jobdef.job.uuid(),
            self.jobdef.job.package().name(),
            self.jobdef.job.package().version()
        ));
        let job_uuid = *self.jobdef.job.uuid();

        // Schedule the job on the scheduler
        match self.scheduler.schedule_job(runnable, self.bar.clone()).await?.run().await? {
            Err(e) => {
                trace!("[{}]: Scheduler returned error = {:?}", self.jobdef.job.uuid(), e);
                // ... and we send that to our parent
                //
                // We only send to one parent, because it doesn't matter anymore
                // We know that we have at least one sender available
                let mut errormap = HashMap::with_capacity(1);
                errormap.insert(job_uuid, e);

                // Every JobTask has at least one sender, so we can [] here.
                self.sender[0]
                    .send(Err(errormap))
                    .await
                    .context("Failed sending scheduler errors to parent")
                    .with_context(|| format!("Failed sending error from job {}", self.jobdef.job.uuid()))?;
                return Ok(())
            },

            // if the scheduler run reports success,
            // it returns the database artifact objects it created!
            Ok(artifacts) => {
                trace!("[{}]: Scheduler returned artifacts = {:?}", self.jobdef.job.uuid(), artifacts);

                // mark the produced artifacts as "built" (rather than reused)
                let artifacts = artifacts.into_iter().map(ProducedArtifact::Built).collect();

                received_dependencies.insert(*self.jobdef.job.uuid(), artifacts);
                for s in self.sender.iter() {
                    s.send(Ok(received_dependencies.clone())).await?;
                }
            },
        }

        trace!("[{}]: Finished successfully", self.jobdef.job.uuid());
        Ok(())
    }

    /// Performe a recv() call on the receiving side of the channel
    ///
    /// Put the dependencies you received into the `received_dependencies`, the errors in the
    /// `received_errors`
    ///
    /// Return Ok(true) if we should continue operation
    /// Return Ok(false) if the channel is empty and we're done receiving or if the channel is
    /// empty and there were errors collected
    async fn perform_receive(&mut self, received_dependencies: &mut HashMap<Uuid, Vec<ProducedArtifact>>, received_errors: &mut HashMap<Uuid, Error>) -> Result<bool> {
        match self.receiver.recv().await {
            Some(Ok(mut v)) => {
                // The task we depend on succeeded and returned an
                // (uuid of the job, [ArtifactPath])
                trace!("[{}]: Received: {:?}", self.jobdef.job.uuid(), v);
                received_dependencies.extend(v);
                Ok(true)
            },
            Some(Err(mut e)) => {
                // The task we depend on failed
                // we log that error for now
                trace!("[{}]: Received: {:?}", self.jobdef.job.uuid(), e);
                received_errors.extend(e);
                Ok(true)
            },
            None => {
                // The task we depend on finished... we must check what we have now...
                trace!("[{}]: Received nothing, channel seems to be empty", self.jobdef.job.uuid());

                // If the channel was closed and there are already errors in the `received_errors`
                // buffer, we return Ok(false) to notify the caller that we should not continue
                // receiving
                if !received_errors.is_empty() {
                    trace!("[{}]: There are errors, stop receiving", self.jobdef.job.uuid());
                    return Ok(false)
                }

                // Find all dependencies that we need but which are not received
                let received = received_dependencies.keys().collect::<Vec<_>>();
                let missing_deps: Vec<_> = self.jobdef
                    .dependencies
                    .iter()
                    .filter(|d| !received.contains(d))
                    .collect();
                trace!("[{}]: Missing dependencies = {:?}", self.jobdef.job.uuid(), missing_deps);

                // ... if there are any, error
                if !missing_deps.is_empty() {
                    let missing: Vec<String> = missing_deps.iter().map(|u| u.to_string()).collect();
                    Err(anyhow!("Childs finished, but dependencies still missing: {:?}", missing))
                } else {
                    // all dependencies are received
                   Ok(false)
                }
            },
        }
    }

}

