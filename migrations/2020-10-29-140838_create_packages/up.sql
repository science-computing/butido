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
CREATE TABLE packages (
    id SERIAL PRIMARY KEY NOT NULL,
    name    VARCHAR NOT NULL,
    version VARCHAR NOT NULL,

    CONSTRAINT UC_name_version UNIQUE (name, version)
)
