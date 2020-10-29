-- Your SQL goes here
CREATE TABLE packages (
    id SERIAL PRIMARY KEY NOT NULL,
    name    VARCHAR NOT NULL,
    version VARCHAR NOT NULL,

    CONSTRAINT UC_name_version UNIQUE (name, version)
)
