use std::path::Path;

use anyhow::anyhow;
use anyhow::Error;
use anyhow::Context;
use anyhow::Result;
use diesel::PgConnection;
use diesel::prelude::*;

use crate::db::models::Job;
use crate::schema::artifacts::*;
use crate::schema::artifacts;

#[derive(Identifiable, Queryable, Associations)]
#[belongs_to(Job)]
pub struct Artifact {
    pub id: i32,
    pub path: String,
    pub released: bool,
    pub job_id: i32,
}

#[derive(Insertable)]
#[table_name="artifacts"]
struct NewArtifact<'a> {
    pub path: &'a str,
    pub released: bool,
    pub job_id: i32,
}

impl Artifact {
    pub fn create(database_connection: &PgConnection, art_path: &Path, art_released: bool, job: &Job) -> Result<Artifact> {
        let path_str = art_path.to_str()
                .ok_or_else(|| anyhow!("Path is not valid UTF-8: {}",  art_path.display()))
                .context("Writing artifact to database")?;
        let new_art = NewArtifact {
            path: path_str,
            released: art_released,
            job_id: job.id,
        };

        diesel::insert_into(artifacts::table)
            .values(&new_art)
            .on_conflict_do_nothing()
            .execute(database_connection)?;

        dsl::artifacts
            .filter(path.eq(path_str))
            .first::<Artifact>(database_connection)
            .map_err(Error::from)
    }
}

