//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use serde::Deserialize;
use serde::Serialize;

#[derive(
    parse_display::Display,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Hash,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
)]
#[display("{0}")]
pub struct ImageName(String);

impl From<String> for ImageName {
    fn from(s: String) -> Self {
        ImageName(s)
    }
}

#[cfg(test)]
impl From<&str> for ImageName {
    fn from(s: &str) -> Self {
        ImageName(String::from(s))
    }
}

impl AsRef<str> for ImageName {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

#[derive(Debug, Deserialize)]
pub struct ContainerImage {
    pub name: ImageName,
    pub short_name: ImageName,
}

#[derive(
    parse_display::Display,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Hash,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
)]
#[serde(transparent)]
#[display("{0}")]
pub struct ContainerHash(String);

impl From<String> for ContainerHash {
    fn from(s: String) -> Self {
        ContainerHash(s)
    }
}

impl AsRef<str> for ContainerHash {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
