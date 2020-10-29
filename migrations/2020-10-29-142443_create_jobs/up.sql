-- Your SQL goes here
CREATE TABLE jobs (
    id SERIAL PRIMARY KEY NOT NULL,

    submit_id 	INTEGER REFERENCES submits(id) NOT NULL,
    endpoint_id INTEGER REFERENCES endpoints(id) NOT NULL,
    package_id 	INTEGER REFERENCES packages(id) NOT NULL,
    image_id 	INTEGER REFERENCES images(id) NOT NULL,
    artifact_id INTEGER REFERENCES artifacts(id) NOT NULL,

    container_hash VARCHAR NOT NULL,
    script_text TEXT NOT NULL,
    log_text TEXT NOT NULL
)
