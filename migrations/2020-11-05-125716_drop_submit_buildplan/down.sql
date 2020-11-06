-- This file should undo anything in `up.sql`
ALTER TABLE
    submits
ADD COLUMN
    buildplan JSONB NOT NULL;
