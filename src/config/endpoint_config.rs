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

#[derive(Clone, Debug, Getters, CopyGetters, Deserialize)]
pub struct Endpoint {
    #[getset(get = "pub")]
    name: String,

    #[getset(get = "pub")]
    uri: String,

    #[getset(get = "pub")]
    endpoint_type: EndpointType,

    /// Relative speed to other endpoints
    ///
    /// So if you have two servers, one with 12 cores and one with 24, you want to set "1" for the
    /// first and "2" for the second (or "12" for the first and "24" for the second - the ratio is
    /// the thing here)!
    #[getset(get_copy = "pub")]
    speed: usize,

    /// Maximum number of jobs which are allowed on this endpoint
    #[getset(get_copy = "pub")]
    maxjobs: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum EndpointType {
    #[serde(rename  = "socket")]
    Socket,
    #[serde(rename  = "http")]
    Http,
}

