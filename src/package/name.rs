use std::ops::Deref;

use pom::parser::Parser as PomParser;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct PackageName(String);

impl Deref for PackageName {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for PackageName {
    fn from(s: String) -> Self {
        PackageName(s)
    }
}

impl std::fmt::Display for PackageName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        self.0.fmt(f)
    }
}

impl PackageName {
    pub fn parser<'a>() -> PomParser<'a, u8, Self> {
        use crate::util::parser::*;
        (letters() + ((letters() | numbers()).repeat(0..)))
            .collect()
            .convert(|b| String::from_utf8(b.to_vec()).map(Self::from))
    }
}

