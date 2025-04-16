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
use anyhow::Context;
use anyhow::Result;
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
#[serde(deny_unknown_fields)]
pub struct ContainerImage {
    pub name: ImageName,
    pub short_name: ImageName,
}

pub struct ImageNameLookup {
    long2short: HashMap<ImageName, ImageName>,
    short2long: HashMap<ImageName, ImageName>,
}

impl ImageNameLookup {
    pub fn create(available_images: &[ContainerImage]) -> Result<Self> {
        let mut long2short = HashMap::new();
        let mut short2long = HashMap::new();
        let mut all_names = HashMap::new(); // Optional (to check for name collisions)
        for (idx, image) in available_images.iter().enumerate() {
            if let Some(duplicate) = all_names.insert(image.name.clone(), idx) {
                return Err(anyhow!(
                    "The image full name \"{0}\" is specified multiple times in the configured `images` list (either as short or full name)",
                    image.name
                )).with_context(|| anyhow!(
                    "The configured container image with index {idx} ({:?}) collides with the previous definition at index {duplicate} ({:?})",
                    available_images.get(idx),
                    available_images.get(duplicate))
                );
            }
            if let Some(duplicate) = all_names.insert(image.short_name.clone(), idx) {
                return Err(anyhow!(
                    "The image short name \"{0}\" is specified multiple times in the configured `images` list (either as short or full name)",
                    image.short_name
                )).with_context(|| anyhow!(
                    "The configured container image with index {idx} ({:?}) collides with the previous definition at index {duplicate} ({:?})",
                    available_images.get(idx),
                    available_images.get(duplicate))
                );
            }
            long2short.insert(image.name.clone(), image.short_name.clone());
            short2long.insert(image.short_name.clone(), image.name.clone());
        }
        Ok(ImageNameLookup {
            long2short,
            short2long,
        })
    }

    // To convert a user-supplied image name into an expanded image name:
    #[rustversion::attr(all(since(1.87), before(1.88)), allow(clippy::map_entry))]
    pub fn expand(&self, image_name: &str) -> Result<ImageName> {
        let image_name = ImageName::from(image_name.to_string());
        if self.long2short.contains_key(&image_name) {
            // It already is a valid long/expanded image name:
            Ok(ImageName::from(image_name.to_string()))
        } else if let Some(image_name) = self.short2long.get(&image_name) {
            Ok(ImageName::from(image_name.to_string()))
        } else {
            // It is neither a valid short name nor a valid long name:
            let available_long_names = self
                .long2short
                .clone()
                .into_keys()
                .map(|name| name.0.to_string());
            let available_short_names = self
                .short2long
                .clone()
                .into_keys()
                .map(|name| name.0.to_string());
            let mut available_images = available_long_names
                .chain(available_short_names)
                .collect::<Vec<_>>();
            available_images.sort_unstable();
            let available_images = available_images.join(",");
            Err(anyhow!("Failed to resolve the requested container image name \"{image_name}\". The available images are: {available_images}"))
        }
    }

    // To try to shorten an image name based on the currently configured short names:
    pub fn shorten(&self, image_name: &str) -> String {
        let image_name = ImageName::from(image_name.to_string());
        self.long2short
            .get(&image_name)
            .unwrap_or(&image_name)
            .to_string()
    }
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
