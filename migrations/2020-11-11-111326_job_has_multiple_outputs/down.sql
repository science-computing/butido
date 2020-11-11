-- This file should undo anything in `up.sql`
ALTER TABLE
    jobs
ADD COLUMN
    artifact_id INTEGER REFERENCES artifacts(id) NOT NULL
