-- Your SQL goes here
CREATE TABLE submits (
    id SERIAL PRIMARY KEY NOT NULL,
    uuid UUID NOT NULL UNIQUE,
    submit_time TIMESTAMP WITH TIME ZONE NOT NULL,

    requested_image_id   INTEGER REFERENCES images(id) NOT NULL,
    requested_package_id INTEGER REFERENCES packages(id) NOT NULL,
    repo_hash_id         INTEGER REFERENCES githashes(id) NOT NULL,

    tree JSONB NOT NULL,
    buildplan JSONB NOT NULL
)
