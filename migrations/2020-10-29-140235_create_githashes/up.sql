-- Your SQL goes here
CREATE TABLE githashes (
    id SERIAL PRIMARY KEY NOT NULL,
    hash VARCHAR(64) NOT NULL UNIQUE
)
