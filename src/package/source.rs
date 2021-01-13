//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::Result;
use anyhow::anyhow;
use getset::Getters;
use log::trace;
use serde::Deserialize;
use serde::Serialize;
use url::Url;

#[derive(Clone, Debug, Serialize, Deserialize, Getters)]
pub struct Source {
    #[getset(get = "pub")]
    url: Url,
    #[getset(get = "pub")]
    hash: SourceHash,
}

impl Source {
    #[cfg(test)]
    pub fn new(url: Url, hash: SourceHash) -> Self {
        Source { url, hash }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Getters)]
pub struct SourceHash {
    #[serde(rename = "type")]
    #[getset(get = "pub")]
    hashtype: HashType,

    #[serde(rename = "hash")]
    #[getset(get = "pub")]
    value: HashValue,
}

impl SourceHash {
    pub fn matches_hash_of(&self, buf: &[u8]) -> Result<()> {
        trace!("Hashing buffer with: {:?}", self.hashtype);
        let h = self.hashtype.hash_buffer(&buf)?;
        trace!("Hashing buffer with: {} finished", self.hashtype);

        if h == self.value {
            trace!("Hash matches expected hash");
            Ok(())
        } else {
            trace!("Hash mismatch expected hash");
            Err(anyhow!("Hash mismatch, expected '{}', got '{}'", self.value, h))
        }
    }

    #[cfg(test)]
    pub fn new(hashtype: HashType, value: HashValue) -> Self {
        SourceHash { hashtype, value }
    }
}


#[derive(parse_display::Display, Clone, Debug, Serialize, Deserialize)]
pub enum HashType {
    #[serde(rename = "sha1")]
    #[display("sha1")]
    Sha1,

    #[serde(rename = "sha256")]
    #[display("sha256")]
    Sha256,

    #[serde(rename = "sha512")]
    #[display("sha512")]
    Sha512,
}

impl HashType {
    fn hash_buffer(&self, buffer: &[u8]) -> Result<HashValue> {
        match self {
            HashType::Sha1 => {
                trace!("SHA1 hashing buffer");
                let mut m = sha1::Sha1::new();
                m.update(buffer);
                Ok(HashValue(m.digest().to_string()))
            },
            HashType::Sha256 => {
                trace!("SHA256 hashing buffer");
                //let mut m = sha2::Sha256::new();
                //m.update(buffer);
                //Ok(HashValue(String::from(m.finalize())))
                unimplemented!()
            },
            HashType::Sha512 => {
                trace!("SHA512 hashing buffer");
                //let mut m = sha2::Sha512::new();
                //m.update(buffer);
                //Ok(HashValue(String::from(m.finalize())))
                unimplemented!()
            },
        }
    }
}


#[derive(parse_display::Display, Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
#[serde(transparent)]
#[display("{0}")]
pub struct HashValue(String);

#[cfg(test)]
impl From<String> for HashValue {
    fn from(s: String) -> Self {
        HashValue(s)
    }
}

