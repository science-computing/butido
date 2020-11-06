-- Your SQL goes here
CREATE TABLE submit_envs (
    id SERIAL PRIMARY KEY NOT NULL,
    submit_id INTEGER REFERENCES submits(id) NOT NULL,
    env_id    INTEGER REFERENCES envvars(id) NOT NULL,

    CONSTRAINT UC_submitid_envid UNIQUE (submit_id, env_id)
)
