--
-- Copyright (c) 2020-2022 science+computing ag and other contributors
--
-- This program and the accompanying materials are made
-- available under the terms of the Eclipse Public License 2.0
-- which is available at https://www.eclipse.org/legal/epl-2.0/
--
-- SPDX-License-Identifier: EPL-2.0
--

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
