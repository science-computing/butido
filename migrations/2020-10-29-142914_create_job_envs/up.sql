-- Your SQL goes here
CREATE TABLE job_envs (
    id SERIAL PRIMARY KEY NOT NULL,
    job_id INTEGER REFERENCES jobs(id) NOT NULL,
    env_id INTEGER REFERENCES envvars(id) NOT NULL,

    CONSTRAINT UC_jobid_envid UNIQUE (job_id, env_id)
)
