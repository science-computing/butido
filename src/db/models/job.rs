use anyhow::Error;
use anyhow::Result;
use diesel::PgConnection;
use diesel::prelude::*;

use crate::db::models::{Submit, Endpoint, Package, Image};
use crate::package::Script;
use crate::schema::jobs::*;
use crate::schema::jobs;
use crate::util::docker::ContainerHash;

#[derive(Queryable)]
pub struct Job {
    pub id: i32,
    pub submit_id: i32,
    pub endpoint_id: i32,
    pub package_id: i32,
    pub image_id: i32,
    pub container_hash: String,
    pub script_text: String,
    pub log_text: String,
}

#[derive(Insertable)]
#[table_name="jobs"]
struct NewJob<'a> {
    pub submit_id: i32,
    pub endpoint_id: i32,
    pub package_id: i32,
    pub image_id: i32,
    pub container_hash: &'a str,
    pub script_text: &'a str,
    pub log_text: &'a str,
}

impl Job {
    pub fn create(database_connection: &PgConnection,
                           submit: &Submit,
                           endpoint: &Endpoint,
                           package: &Package,
                           image: &Image,
                           container: &ContainerHash,
                           script: &Script,
                           log: &str,
                           ) -> Result<()> {
        let new_job = NewJob {
            submit_id: submit.id,
            endpoint_id: endpoint.id,
            package_id: package.id,
            image_id: image.id,
            container_hash: container.as_ref(),
            script_text: script.as_ref(),
            log_text: log,
        };

        diesel::insert_into(jobs::table)
            .values(&new_job)
            .on_conflict_do_nothing()
            .execute(database_connection)
            .map_err(Error::from)
            .map(|_| ())
    }
}

