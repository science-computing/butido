use std::ops::Deref;

use pom::parser::Parser as PomParser;
use serde::Deserialize;
use serde::Serialize;

#[derive(parse_display::Display, Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
#[display("{0}")]
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

impl PackageName {
    pub fn parser<'a>() -> PomParser<'a, u8, Self> {
        use crate::util::parser::*;
        (letters() + ((letters() | numbers()).repeat(0..)))
            .collect()
            .convert(|b| String::from_utf8(b.to_vec()).map(Self::from))
    }
}

