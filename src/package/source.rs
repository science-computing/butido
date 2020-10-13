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

#[derive(Clone, Debug, Deserialize)]
pub struct SourceHash {
    #[serde(rename = "type")]
    hashtype: HashType,

    #[serde(rename = "hash")]
    value: HashValue,
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

#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq)]
#[serde(transparent)]
pub struct HashValue(String);

