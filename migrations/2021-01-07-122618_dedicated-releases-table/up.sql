-- Your SQL goes here
ALTER TABLE
    artifacts
DROP COLUMN
    released;

CREATE TABLE releases (
    id SERIAL PRIMARY KEY NOT NULL,
    artifact_id INTEGER REFERENCES artifacts(id) NOT NULL,
    release_date TIMESTAMP WITH TIME ZONE NOT NULL,

    CONSTRAINT UC_art_release_unique UNIQUE (artifact_id, release_date)
);
