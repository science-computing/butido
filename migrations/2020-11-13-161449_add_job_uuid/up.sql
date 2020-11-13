-- Your SQL goes here
ALTER TABLE
    jobs
ADD COLUMN
    uuid UUID NOT NULL UNIQUE
