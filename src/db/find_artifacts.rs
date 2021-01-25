//
// Copyright (c) 2020-2021 science+computing ag and other contributors
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
use anyhow::Error;
use anyhow::Result;
use chrono::NaiveDateTime;
use diesel::BoolExpressionMethods;
use diesel::ExpressionMethods;
use diesel::JoinOnDsl;
use diesel::NullableExpressionMethods;
use diesel::PgConnection;
use diesel::QueryDsl;
use diesel::RunQueryDsl;
use log::trace;
use resiter::AndThen;
use resiter::Map;

use crate::config::Configuration;
use crate::db::models as dbmodels;
use crate::filestore::path::ArtifactPath;
use crate::filestore::path::FullArtifactPath;
use crate::filestore::ReleaseStore;
use crate::filestore::StagingStore;
use crate::package::Package;
use crate::package::ParseDependency;
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
    release_store: &'a ReleaseStore,
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
    let build_dependencies_names = pkg
        .dependencies()
        .build()
        .iter()
        .map(|d| d.parse_as_name_and_version())
        .map_ok(|tpl| tpl.0) // TODO: We only filter by dependency NAME right now, not by version constraint
        .collect::<Result<Vec<_>>>()?;

    let runtime_dependencies_names = pkg
        .dependencies()
        .runtime()
        .iter()
        .map(|d| d.parse_as_name_and_version())
        .map_ok(|tpl| tpl.0) // TODO: We only filter by dependency NAME right now, not by version constraint
        .collect::<Result<Vec<_>>>()?;

    trace!("Build dependency names: {:?}", build_dependencies_names);
    trace!("Runtime dependency names: {:?}", runtime_dependencies_names);

    let mut query = schema::submits::table
        .inner_join(schema::jobs::table)
        .inner_join(schema::packages::table)
        .filter({
            // The package with pkg.name() and pkg.version()
            let package_name_filter = schema::packages::name.eq(pkg.name().as_ref() as &str);
            let package_version_filter =
                schema::packages::version.eq(pkg.version().as_ref() as &str);

            let dependency_filter = {
                // Filter for dependencies
                //
                // We select only packages where the submit contained a job for the
                // dependencies (by name for now).
                let build_refs = build_dependencies_names
                    .iter()
                    .map(AsRef::<str>::as_ref)
                    .collect::<Vec<_>>();
                let runtime_refs = runtime_dependencies_names
                    .iter()
                    .map(AsRef::<str>::as_ref)
                    .collect::<Vec<_>>();
                schema::packages::name
                    .eq_any(build_refs)
                    .or(schema::packages::name.eq_any(runtime_refs))
            };

            package_name_filter
                .and(package_version_filter)
                .or(dependency_filter)
        })
        .inner_join(schema::artifacts::table.on(schema::jobs::id.eq(schema::artifacts::job_id)))
        .left_join(
            schema::releases::table.on(schema::releases::artifact_id.eq(schema::artifacts::id)),
        )
        .inner_join({
            schema::job_envs::table
                .inner_join(schema::envvars::table)
                .on(schema::jobs::id.eq(schema::job_envs::job_id))
        })
        .inner_join(schema::images::table)
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
            let rels = schema::releases::release_date.nullable();

            (arts, jobs, rels)
        })
        .load::<(dbmodels::Artifact, dbmodels::Job, Option<NaiveDateTime>)>(
            &*database_connection,
        )
        .map_err(Error::from)
        .and_then(|results: Vec<_>| {
            results
                .into_iter()
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
                .map(|tpl| -> Result<(_, _, _)> {
                    // This is a Iterator::filter() but because our condition here might fail, we
                    // map() and do the actual filtering later.

                    let job = tpl.1;
                    let job_env: Vec<(String, String)> = job
                        .env(&*database_connection)?
                        .into_iter()
                        .map(|var: dbmodels::EnvVar| (var.name, var.value))
                        .collect();

                    trace!("The job we found had env: {:?}", job_env);
                    if let Some(pkg_env) = package_environment.as_ref() {
                        let filter_result = job_env.iter()
                            .all(|(k, v)| {
                                pkg_env
                                    .iter()
                                    .chain(additional_env.iter().map(|tpl| (&tpl.0, &tpl.1)))
                                    .any(|(key, value)| k == key.as_ref() && v == value)
                            });

                        Ok((tpl.0, filter_result, tpl.2))
                    } else {
                        Ok((tpl.0, true, tpl.2))
                    }
                })
                .filter(|r| match r { // the actual filtering from above
                    Err(_)         => true,
                    Ok((_, bl, _)) => *bl,
                })
                .and_then_ok(|(p, _, ndt)| ArtifactPath::new(PathBuf::from(p.path)).map(|a| (a, ndt)))
                .and_then_ok(|(artpath, ndt)| {
                    if let Some(staging) = staging_store.as_ref() {
                        trace!(
                            "Searching in staging: {:?} for {:?}",
                            staging.root_path(),
                            artpath
                        );
                        if let Some(art) = staging.get(&artpath) {
                            trace!("Found in staging: {:?}", art);
                            return staging.root_path().join(art).map(|p| (p, ndt));
                        }
                    }

                    let art = release_store
                        .get(&artpath)
                        .ok_or_else(|| anyhow!("Failed to find artifact for: {:?}", artpath))?;
                    trace!("Found in release: {:?}", art);
                    release_store.root_path().join(art).map(|p| (p, ndt))
                })
                .collect::<Result<Vec<(FullArtifactPath<'a>, Option<NaiveDateTime>)>>>()
        })
}
