-- Your SQL goes here
ALTER TABLE
    artifacts
ADD COLUMN
    job_id INTEGER REFERENCES jobs(id) NOT NULL
