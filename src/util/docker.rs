//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::collections::HashMap;

use anyhow::anyhow;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use tracing::warn;

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
#[serde(deny_unknown_fields)]
pub struct ContainerImage {
    pub name: ImageName,
    pub short_name: ImageName,
}

// To convert a user-supplied image name into an expanded image name:
pub fn resolve_image_name(name: &str, available_images: &Vec<ContainerImage>) -> Result<ImageName> {
    let mut images = HashMap::new();
    for image in available_images {
        if images.insert(&image.name, &image.name).is_some() {
            warn!(
                "The image name \"{0}\" is specified multiple times in the configured `images` list",
                image.name
            );
        }
        if images.insert(&image.short_name, &image.name).is_some() {
            warn!(
                "The image short name \"{0}\" is specified multiple times in the configured `images` list",
                image.short_name
            );
        }
    }
    images.get(&ImageName::from(name.to_string())).cloned().ok_or_else(|| {
        let mut available_images = images.into_keys().map(|name| name.0.to_string()).collect::<Vec<_>>();
        available_images.sort_unstable();
        let available_images = available_images.join(",");
        anyhow!("Failed to resolve the requested container image name \"{name}\". The available images are: {available_images}")
    }).cloned()
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
