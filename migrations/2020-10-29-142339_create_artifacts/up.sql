-- Your SQL goes here
CREATE TABLE artifacts (
    id SERIAL PRIMARY KEY NOT NULL,
    path VARCHAR NOT NULL UNIQUE
)
