use anyhow::Result;
use diesel::PgConnection;
use diesel::prelude::*;

use crate::schema::job_envs::*;
use crate::schema::job_envs;
use crate::db::models::{Job, EnvVar};

#[derive(Queryable)]
pub struct JobEnv {
    pub id: i32,
    pub job_id: i32,
    pub env_id: i32,
}

#[derive(Insertable)]
#[table_name="job_envs"]
struct NewJobEnv {
    pub job_id: i32,
    pub env_id: i32,
}

impl JobEnv {
    pub fn create(database_connection: &PgConnection, job: &Job, env: &EnvVar) -> Result<()> {
        let new_jobenv = NewJobEnv { job_id: job.id, env_id: env.id };

        diesel::insert_into(job_envs::table)
            .values(&new_jobenv)
            .execute(database_connection)?;
        Ok(())
    }
}

