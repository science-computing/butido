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
