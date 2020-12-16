use serde::Deserialize;
use serde::Serialize;

#[derive(parse_display::Display ,Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[display("{0}")]
pub struct ImageName(String);

impl From<String> for ImageName {
    fn from(s: String) -> Self {
        ImageName(s)
    }
}

impl AsRef<str> for ImageName {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}


#[derive(parse_display::Display, Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
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

