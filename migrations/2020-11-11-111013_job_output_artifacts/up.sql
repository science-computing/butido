--
-- Copyright (c) 2020-2021 science+computing ag and other contributors
--
-- This program and the accompanying materials are made
-- available under the terms of the Eclipse Public License 2.0
-- which is available at https://www.eclipse.org/legal/epl-2.0/
--
-- SPDX-License-Identifier: EPL-2.0
--

-- Your SQL goes here
CREATE TABLE job_output_artifacts (
    id SERIAL PRIMARY KEY NOT NULL,
    job_id      INTEGER REFERENCES jobs(id) NOT NULL,
    artifact_id INTEGER REFERENCES artifacts(id) NOT NULL,

    CONSTRAINT UC_jobid_output_artifactid UNIQUE (job_id, artifact_id)
)
