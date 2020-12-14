-- Your SQL goes here
ALTER TABLE
    artifacts
ADD CONSTRAINT
    path_job_id_unique
    UNIQUE (path, job_id)
