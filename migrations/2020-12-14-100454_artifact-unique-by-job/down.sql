-- This file should undo anything in `up.sql`
ALTER TABLE
    artifacts
DROP CONSTRAINT
    path_job_id_unique
