-- Your SQL goes here
CREATE TABLE job_input_artifacts (
    id SERIAL PRIMARY KEY NOT NULL,
    job_id      INTEGER REFERENCES jobs(id) NOT NULL,
    artifact_id INTEGER REFERENCES artifacts(id) NOT NULL,

    CONSTRAINT UC_jobid_artifactid UNIQUE (job_id, artifact_id)
)
