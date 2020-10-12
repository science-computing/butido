use anyhow::Result;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;

pub trait VersionParser {
    fn parse(&self, buffer: &dyn NameVersionBuffer) -> Result<(PackageName, PackageVersionConstraint)>;
}

pub trait NameVersionBuffer {
    fn get_as_str(&self) -> &str;
}

pub struct DummyVersionParser;
impl DummyVersionParser {
    pub fn new() -> Self {
        DummyVersionParser
    }
}

impl VersionParser for DummyVersionParser {
    fn parse(&self, buffer: &dyn NameVersionBuffer) -> Result<(PackageName, PackageVersionConstraint)> {
        let v: Vec<_> = buffer.get_as_str().split("-").collect();
        Ok((PackageName::from(String::from(v[0])), PackageVersionConstraint::Any))
    }
}

