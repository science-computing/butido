use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
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

impl std::fmt::Display for ImageName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        self.0.fmt(f)
    }
}


#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
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

impl std::fmt::Display for ContainerHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        self.0.fmt(f)
    }
}

