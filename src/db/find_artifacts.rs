//
// Copyright (c) 2020-2021 science+computing ag and other contributors
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

use anyhow::Error;
use anyhow::Result;
use chrono::NaiveDateTime;
use diesel::BoolExpressionMethods;
use diesel::ExpressionMethods;
use diesel::JoinOnDsl;
use diesel::PgConnection;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use log::trace;
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
pub fn find_artifacts<'a>(
    database_connection: Arc<PgConnection>,
    config: &Configuration,
    pkg: &Package,
    release_stores: &'a [Arc<ReleaseStore>],
    staging_store: Option<&'a StagingStore>,
    additional_env: &[(EnvironmentVariableName, String)],
    script_filter: bool,
) -> Result<Vec<(FullArtifactPath<'a>, Option<NaiveDateTime>)>> {
    let shebang = Shebang::from(config.shebang().clone());
    let script = if script_filter {
        let script = ScriptBuilder::new(&shebang).build(
            pkg,
            config.available_phases(),
            *config.strict_script_interpolation(),
        )?;
        Some(script)
    } else {
        None
    };

    let package_environment = pkg.environment();
    let mut query = schema::packages::table
        .filter({
            // The package with pkg.name() and pkg.version()
            let package_name_filter = schema::packages::name.eq(pkg.name().as_ref() as &str);
            let package_version_filter =
                schema::packages::version.eq(pkg.version().as_ref() as &str);

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

    if let Some(allowed_images) = pkg.allowed_images() {
        trace!("Filtering with allowed_images = {:?}", allowed_images);
        let imgs = allowed_images
            .iter()
            .map(AsRef::<str>::as_ref)
            .collect::<Vec<_>>();
        query = query.filter(schema::images::name.eq_any(imgs));
    }

    if let Some(denied_images) = pkg.denied_images() {
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

    trace!("Query = {}", diesel::debug_query(&query));

    query
        .select({
            let arts = schema::artifacts::all_columns;
            let jobs = schema::jobs::all_columns;
            //let rels = schema::releases::release_date.nullable();

            (arts, jobs)
        })
        .load::<(dbmodels::Artifact, dbmodels::Job)>(
            &*database_connection,
        )
        .map_err(Error::from)
        .and_then(|results: Vec<_>| {
            results
                .into_iter()
                .inspect(|(art, job)| log::debug!("Filtering further: {:?}, job {:?}", art, job.id))
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
                        .env(&*database_connection)?
                        .into_iter()
                        .map(|var: dbmodels::EnvVar| (var.name, var.value))
                        .collect();

                    trace!("The job we found had env: {:?}", job_env);
                    let envs_equal = environments_equal(&job_env, package_environment.as_ref(), additional_env);
                    trace!("environments where equal = {}", envs_equal);
                    Ok((tpl.0, envs_equal))
                })
                .filter(|r| match r { // the actual filtering from above
                    Err(_)         => true,
                    Ok((_, bl)) => *bl,
                })
                .and_then_ok(|(art, _)| {
                    if let Some(release) = art.get_release(&*database_connection)? {
                        Ok((art, Some(release.release_date)))
                    } else {
                        Ok((art, None))
                    }
                })
                .and_then_ok(|(p, ndt)| ArtifactPath::new(PathBuf::from(p.path)).map(|a| (a, ndt)))
                .and_then_ok(|(artpath, ndt)| {
                    if let Some(staging) = staging_store.as_ref() {
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
                    for release_store in release_stores {
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
        })
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

