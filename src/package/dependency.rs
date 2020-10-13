use std::result::Result as RResult;
use serde::Deserialize;

use crate::package::PackageName;
use crate::package::PackageVersionConstraint;


#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct SystemDependency(String);

#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct BuildDependency(String);

#[derive(Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct Dependency(String);

impl std::convert::TryInto<(PackageName, PackageVersionConstraint)> for Dependency {
    type Error = anyhow::Error;

    fn try_into(self) -> RResult<(PackageName, PackageVersionConstraint), Self::Error> {
        // TODO: Implement properly
        let v: Vec<_> = self.0.split("-").collect();
        Ok((PackageName::from(String::from(v[0])), PackageVersionConstraint::Any))
    }
}

