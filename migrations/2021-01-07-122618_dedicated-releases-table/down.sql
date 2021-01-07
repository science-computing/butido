-- This file should undo anything in `up.sql`
ALTER TABLE
    artifacts
ADD COLUMN
    released boolean NOT NULL;

DROP TABLE releases;
