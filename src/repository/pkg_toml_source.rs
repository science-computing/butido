// Copyright (c) 2020-2024 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0

// A custom `Source` implementation for the `config` crate to tack the `pkg.toml` file path as URI/origin
// in addition to the content (basically a replacement for `config::File::from_str(str, format)`).

use std::path::Path;

use config::ConfigError;
use config::FileFormat;
use config::Format;
use config::Map;
use config::Source;
use config::Value;

#[derive(Clone, Debug)]
pub struct PkgTomlSource {
    content: String,
    uri: String,
}

impl PkgTomlSource {
    pub fn new(path: &Path, content: String) -> Self {
        // We could also use `path.to_str()` for proper error handling:
        let path = path.to_string_lossy().to_string();
        PkgTomlSource { content, uri: path }
    }
}

impl Source for PkgTomlSource {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new((*self).clone())
    }

    fn collect(&self) -> Result<Map<String, Value>, ConfigError> {
        FileFormat::Toml
            .parse(Some(&self.uri), &self.content)
            .map_err(|cause| ConfigError::FileParse {
                uri: Some(self.uri.clone()),
                cause,
            })
    }
}
