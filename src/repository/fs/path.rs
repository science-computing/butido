//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::path::Component;

use anyhow::anyhow;
use anyhow::Result;

/// Helper type for filtering for paths we need or don't need
///
/// We either have a directory, which has a name, or we have a pkg.toml file, which is of interest.
/// All other files can be ignored and thus are not represented by this type.
///
/// The PathComponent::DirName(_) represents a _part_ of a Path. Something like
///
/// ```ignore
///     let p = PathBuf::from("foo/bar/baz")
///     p.components().map(PathComponent::DirName) // does not actually work because of types
/// ```
///
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathComponent {
    PkgToml,
    DirName(String),
}

impl TryFrom<&std::path::Component<'_>> for PathComponent {
    type Error = anyhow::Error;

    fn try_from(c: &std::path::Component) -> Result<Self> {
        match *c {
            Component::Prefix(_) => anyhow::bail!("Unexpected path component: Prefix"),
            Component::RootDir => anyhow::bail!("Unexpected path component: RootDir"),
            Component::CurDir => anyhow::bail!("Unexpected path component: CurDir"),
            Component::ParentDir => anyhow::bail!("Unexpected path component: ParentDir"),
            Component::Normal(filename) => {
                let filename = filename.to_str().ok_or_else(|| anyhow!("UTF8-error"))?;
                if filename == "pkg.toml" {
                    Ok(PathComponent::PkgToml)
                } else {
                    Ok(PathComponent::DirName(filename.to_string()))
                }
            }
        }
    }
}

impl PathComponent {
    /// Helper fn to get the directory name of this PathComponent if it is a PathComponent::DirName
    /// or None if it is not.
    pub fn dir_name(&self) -> Option<&str> {
        match self {
            PathComponent::PkgToml => None,
            PathComponent::DirName(dn) => Some(dn),
        }
    }
}
