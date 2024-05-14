//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use getset::{CopyGetters, Getters};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[serde(transparent)]
pub struct EndpointName(String);

impl From<String> for EndpointName {
    fn from(s: String) -> Self {
        EndpointName(s)
    }
}

impl std::fmt::Display for EndpointName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        self.0.fmt(f)
    }
}

impl AsRef<str> for EndpointName {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl EndpointName {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// Configuration of a single endpoint
#[derive(Clone, Debug, Getters, CopyGetters, Deserialize)]
pub struct Endpoint {
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

    /// Duration length of timeout for connecting endpoint
    #[getset(get = "pub")]
    timeout: Option<u64>,
}

/// The type of an endpoint
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum EndpointType {
    #[serde(rename = "socket")]
    Socket,
    #[serde(rename = "http")]
    Http,
}
