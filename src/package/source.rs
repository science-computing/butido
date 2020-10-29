use url::Url;
use serde::Deserialize;
use getset::Getters;

#[derive(Clone, Debug, Deserialize, Getters)]
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

#[derive(Clone, Debug, Deserialize, Getters)]
pub struct SourceHash {
    #[serde(rename = "type")]
    #[getset(get = "pub")]
    hashtype: HashType,

    #[serde(rename = "hash")]
    #[getset(get = "pub")]
    value: HashValue,
}

impl SourceHash {
    #[cfg(test)]
    pub fn new(hashtype: HashType, value: HashValue) -> Self {
        SourceHash { hashtype, value }
    }
}


#[derive(Clone, Debug, Deserialize)]
pub enum HashType {
    #[serde(rename = "sha1")]
    Sha1,

    #[serde(rename = "sha256")]
    Sha256,

    #[serde(rename = "sha512")]
    Sha512,
}

impl std::fmt::Display for HashType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            HashType::Sha1   => write!(f, "sha1"),
            HashType::Sha256 => write!(f, "sha256"),
            HashType::Sha512 => write!(f, "sha512"),
        }
    }
}


#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
#[serde(transparent)]
pub struct HashValue(String);

#[cfg(test)]
impl From<String> for HashValue {
    fn from(s: String) -> Self {
        HashValue(s)
    }
}

impl std::fmt::Display for HashValue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        self.0.fmt(f)
    }
}

