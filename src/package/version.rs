use anyhow::Result;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;

pub trait VersionParser {
    fn parse(&self, buffer: &dyn NameVersionBuffer) -> Result<(PackageName, PackageVersionConstraint)>;
}

pub trait NameVersionBuffer {
    fn get_as_str(&self) -> &str;
}

