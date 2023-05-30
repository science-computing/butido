//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::Result;
use diesel::prelude::*;
use diesel::PgConnection;

use crate::db::models::EnvVar;
use crate::db::models::Job;
use crate::schema::job_envs;

#[derive(Identifiable, Queryable, Associations)]
#[diesel(belongs_to(Job))]
#[diesel(belongs_to(EnvVar, foreign_key = env_id))]
#[diesel(table_name = job_envs)]
pub struct JobEnv {
    pub id: i32,
    pub job_id: i32,
    pub env_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = job_envs)]
struct NewJobEnv {
    pub job_id: i32,
    pub env_id: i32,
}

impl JobEnv {
    pub fn create(database_connection: &mut PgConnection, job: &Job, env: &EnvVar) -> Result<()> {
        let new_jobenv = NewJobEnv {
            job_id: job.id,
            env_id: env.id,
        };

        diesel::insert_into(job_envs::table)
            .values(&new_jobenv)
            .execute(database_connection)?;
        Ok(())
    }
}
