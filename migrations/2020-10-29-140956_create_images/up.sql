-- Your SQL goes here
CREATE TABLE images (
    id SERIAL PRIMARY KEY NOT NULL,
    name VARCHAR NOT NULL UNIQUE
)
