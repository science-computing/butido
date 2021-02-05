-- This file should undo anything in `up.sql`
ALTER TABLE
    submits
ADD COLUMN
    tree JSONB NOT NULL
