//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use chrono::NaiveDateTime;
use diesel::BoolExpressionMethods;
use diesel::ExpressionMethods;
use diesel::JoinOnDsl;
use diesel::PgConnection;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use diesel::r2d2::ConnectionManager;
use diesel::r2d2::Pool;
use tracing::{debug, trace};
use resiter::AndThen;
use resiter::FilterMap;

use crate::config::Configuration;
use crate::db::models as dbmodels;
use crate::filestore::path::ArtifactPath;
use crate::filestore::path::FullArtifactPath;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::package::Package;
use crate::package::ScriptBuilder;
use crate::package::Shebang;
use crate::schema;
use crate::util::EnvironmentVariableName;
use crate::util::docker::ImageName;

/// Find an artifact by a job description
///
/// This function finds artifacts for a job description and environment that is equal to the passed
/// one.
/// The package is not the only parameter that influences a build, so this function gets all the
/// things: The Package, the Release store, the Staging store (optionally), additional environment
/// variables,...
/// to find artifacts for a job that looks the very same.
///
/// If the artifact was released, the return value contains a Some(NaiveDateTime), marking the date
/// of the release.
/// Releases are returned prefferably, if multiple equal pathes for an artifact are found.
#[derive(typed_builder::TypedBuilder)]
pub struct FindArtifacts<'a> {
    config: &'a Configuration,
    database_pool: Pool<ConnectionManager<PgConnection>>,

    /// The release stores to search in
    release_stores: &'a [Arc<ReleaseStore>],

    /// The staging store to search in, if any
    #[builder(default)]
    staging_store: Option<&'a StagingStore>,

    /// Whether to apply a filter that matches for equal script
    ///
    /// If a job can be found, but the script is not equal to the script of the found, the job is
    /// not returned
    script_filter: bool,

    /// Filter for these environment variables
    env_filter: &'a [(EnvironmentVariableName, String)],

    /// Filter for image name
    #[builder(default)]
    image_name: Option<&'a ImageName>,

    /// Search for this package
    package: &'a Package,
}

impl<'a> FindArtifacts<'a> {
    /// Run the FindArtifact as configured
    pub fn run(self) -> Result<Vec<(FullArtifactPath<'a>, Option<NaiveDateTime>)>> {
        let shebang = Shebang::from(self.config.shebang().clone());
        let script = if self.script_filter {
            let script = ScriptBuilder::new(&shebang).build(
                self.package,
                self.config.available_phases(),
                *self.config.strict_script_interpolation(),
            )?;
            Some(script)
        } else {
            None
        };

        let package_environment = self.package.environment();
        let mut query = schema::packages::table
            .filter({
                // The package with pkg.name() and pkg.version()
                let package_name_filter = schema::packages::name.eq(self.package.name().as_ref() as &str);
                let package_version_filter =
                    schema::packages::version.eq(self.package.version().as_ref() as &str);

                package_name_filter.and(package_version_filter)
            })

            // TODO: Only select from submits where the submit contained jobs that are in the
            // dependencies of `pkg`.
            .inner_join(schema::jobs::table.inner_join(schema::submits::table))
            .inner_join(schema::artifacts::table.on(schema::jobs::id.eq(schema::artifacts::job_id)))

            // TODO: We do not yet have a method to "left join" properly, because diesel only has
            // left_outer_join (left_join is an alias)
            // So do not include release dates here, for now
            //.left_outer_join(schema::releases::table.on(schema::releases::artifact_id.eq(schema::artifacts::id)))
            .inner_join(schema::images::table.on(schema::submits::requested_image_id.eq(schema::images::id)))
            .into_boxed();

        if let Some(allowed_images) = self.package.allowed_images() {
            trace!("Filtering with allowed_images = {:?}", allowed_images);
            let imgs = allowed_images
                .iter()
                .map(AsRef::<str>::as_ref)
                .collect::<Vec<_>>();
            query = query.filter(schema::images::name.eq_any(imgs));
        }

        if let Some(denied_images) = self.package.denied_images() {
            trace!("Filtering with denied_images = {:?}", denied_images);
            let imgs = denied_images
                .iter()
                .map(AsRef::<str>::as_ref)
                .collect::<Vec<_>>();
            query = query.filter(schema::images::name.ne_all(imgs));
        }

        if let Some(script_text) = script.as_ref() {
            query = query.filter(schema::jobs::script_text.eq(script_text.as_ref()));
        }

        if let Some(image_name) = self.image_name.as_ref() {
            query = query.filter(schema::images::name.eq(image_name.as_ref()));
        }

        trace!("Query = {}", diesel::debug_query(&query));

        query
            .select({
                let arts = schema::artifacts::all_columns;
                let jobs = schema::jobs::all_columns;
                //let rels = schema::releases::release_date.nullable();

                (arts, jobs)
            })
            .load::<(dbmodels::Artifact, dbmodels::Job)>(&mut self.database_pool.get().unwrap())?
            .into_iter()
            .inspect(|(art, job)| debug!("Filtering further: {:?}, job {:?}", art, job.id))
            //
            // Filter by environment variables
            // All environment variables of the package must be present in the loaded
            // package, so that we can be sure that the loaded package was built with
            // the same ENV.
            //
            // TODO:
            // Doing this in the database query would be way nicer, but I was not able
            // to implement it.
            //
            .map(|tpl| -> Result<(_, _)> {
                // This is a Iterator::filter() but because our condition here might fail, we
                // map() and do the actual filtering later.

                let job = tpl.1;
                let job_env: Vec<(String, String)> = job
                    .env(&mut self.database_pool.get().unwrap())?
                    .into_iter()
                    .map(|var: dbmodels::EnvVar| (var.name, var.value))
                    .collect();

                trace!("The job we found had env: {:?}", job_env);
                let envs_equal = environments_equal(&job_env, package_environment.as_ref(), self.env_filter);
                trace!("environments where equal = {}", envs_equal);
                Ok((tpl.0, envs_equal))
            })
            .filter(|r| match r { // the actual filtering from above
                Err(_)         => true,
                Ok((_, bl)) => *bl,
            })
            .and_then_ok(|(art, _)| {
                if let Some(release) = art.get_release(&mut self.database_pool.get().unwrap())? {
                    Ok((art, Some(release.release_date)))
                } else {
                    Ok((art, None))
                }
            })
            .and_then_ok(|(p, ndt)| ArtifactPath::new(PathBuf::from(p.path)).map(|a| (a, ndt)))
            .and_then_ok(|(artpath, ndt)| {
                if let Some(staging) = self.staging_store.as_ref() {
                    trace!(
                        "Searching in staging: {:?} for {:?}",
                        staging.root_path(),
                        artpath
                    );
                    if let Some(art) = staging.get(&artpath) {
                        trace!("Found in staging: {:?}", art);
                        return staging.root_path().join(art).map(|p| p.map(|p| (p, ndt)))
                    }
                }

                // If we cannot find the artifact in the release store either, we return None.
                // This is the case if there indeed was a release, but it was removed from the
                // filesystem.
                for release_store in self.release_stores {
                    if let Some(art) = release_store.get(&artpath) {
                        trace!("Found in release: {:?}", art);
                        return release_store.root_path().join(art).map(|p| p.map(|p| (p, ndt)))
                    }
                }

                trace!("Found no release for artifact {:?} in any release store", artpath.display());
                Ok(None)
            })
            .filter_map_ok(|opt| opt)
            .collect::<Result<Vec<(FullArtifactPath<'a>, Option<NaiveDateTime>)>>>()
    }
}


fn environments_equal(job_env: &[(String, String)], pkg_env: Option<&HashMap<EnvironmentVariableName, String>>, add_env: &[(EnvironmentVariableName, String)]) -> bool {
    use std::ops::Deref;

    let job_envs_all_found = || job_env.iter()
        .map(|(key, value)| (EnvironmentVariableName::from(key.deref()), value))
        .all(|(key, value)| {

            // check whether pair is in pkg_env
            let is_in_pkg_env = || pkg_env.as_ref()
                .map(|hm| {
                    if let Some(val) = hm.get(&key) {
                        value == val
                    } else {
                        false
                    }
                })
                .unwrap_or(false);

            // check whether pair is in add_env
            let is_in_add_env = || add_env.iter().any(|(k, v)| *k == key && v == value);

            let r = is_in_pkg_env() || is_in_add_env();
            trace!("Job Env ({}, {}) found: {}", key, value, r);
            r
        });

    let pkg_envs_all_found = || pkg_env.map(|hm| {
        hm.iter()
            .all(|(k, v)| {
            job_env.contains(&(k.as_ref().to_string(), v.to_string())) // TODO: do not allocate
            })
    })
    .unwrap_or(true);

    let add_envs_all_found = || add_env.iter()
        .all(|(k, v)| {
            job_env.contains(&(k.as_ref().to_string(), v.to_string())) // TODO: do not allocate
        });

    job_envs_all_found() && pkg_envs_all_found() && add_envs_all_found()
}

