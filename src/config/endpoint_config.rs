//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use getset::{CopyGetters, Getters};
use serde::Deserialize;

/// Configuration of a single endpoint
#[derive(Clone, Debug, Getters, CopyGetters, Deserialize)]
pub struct Endpoint {
    /// A human-readable name of the endpoint
    #[getset(get = "pub")]
    name: String,

    /// The URI where the endpoint is reachable
    #[getset(get = "pub")]
    uri: String,

    /// The type of the endpoint
    #[getset(get = "pub")]
    endpoint_type: EndpointType,

    /// Maximum number of jobs which are allowed on this endpoint
    #[getset(get_copy = "pub")]
    maxjobs: usize,

    #[getset(get = "pub")]
    network_mode: Option<String>,
}

/// The type of an endpoint
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum EndpointType {
    #[serde(rename = "socket")]
    Socket,
    #[serde(rename = "http")]
    Http,
}
