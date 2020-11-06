-- Your SQL goes here
CREATE TABLE envvars (
    id SERIAL PRIMARY KEY NOT NULL,
    name VARCHAR NOT NULL,
    value VARCHAR NOT NULL,

    CONSTRAINT UC_name_value UNIQUE (name, value)
)
