-- Your SQL goes here
CREATE TABLE endpoints (
    id SERIAL PRIMARY KEY NOT NULL,
    name VARCHAR NOT NULL UNIQUE
)
