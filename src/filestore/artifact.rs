use std::cmp::Ord;
use std::cmp::Ordering;
use std::cmp::PartialOrd;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use getset::Getters;
use pom::parser::Parser as PomParser;

use crate::package::PackageName;
use crate::package::PackageVersion;

#[derive(Clone, PartialEq, Eq, Debug, Getters)]
pub struct Artifact {
    #[getset(get = "pub")]
    path: PathBuf,

    #[getset(get = "pub")]
    name: PackageName,

    #[getset(get = "pub")]
    version: PackageVersion,
}

impl PartialOrd for Artifact {
    fn partial_cmp(&self, other: &Artifact) -> Option<Ordering> {
        self.version.partial_cmp(&other.version)
    }
}

impl Ord for Artifact {
    fn cmp(&self, other: &Self) -> Ordering {
        self.version.cmp(&other.version)
    }
}


impl Artifact {
    pub fn load(root: &Path, path: &Path) -> Result<Self> {
        let joined = root.join(path);
        if joined.is_file() {
            let (name, version) = Self::parse_path(root, path)
                .with_context(|| anyhow!("Pathing artifact path: '{}'", joined.display()))?;

            Ok(Artifact {
                path: path.to_path_buf(),

                name,
                version
            })
        } else {
            if path.is_dir() {
                Err(anyhow!("Cannot load non-file path: {}", path.display()))
            } else {
                Err(anyhow!("Path does not exist: {}", path.display()))
            }
        }
    }

    fn parse_path(root: &Path, path: &Path) -> Result<(PackageName, PackageVersion)> {
        path.file_stem()
            .ok_or_else(|| anyhow!("Cannot get filename from {}", (root.join(path)).display()))?
            .to_owned()
            .into_string()
            .map_err(|_| anyhow!("Internal conversion of '{}' to UTF-8", (root.join(path)).display()))
            .and_then(|s| Self::parser().parse(s.as_bytes()).map_err(Error::from))
    }

    /// Construct a parser that parses a Vec<u8> into (PackageName, PackageVersion)
    fn parser<'a>() -> PomParser<'a, u8, (PackageName, PackageVersion)> {
        (PackageName::parser() + crate::util::parser::dash() + PackageVersion::parser())
            .map(|((name, _), vers)| (name, vers))
    }

    pub fn create(root: &Path, name: PackageName, version: PackageVersion) -> Result<Self> {
        let path = Self::create_path(root, &name, &version)?;
        if !path.exists() {
            Ok(Artifact {
                path,
                name,
                version
            })
        } else {
            Err(anyhow!("Path exists: {}", path.display()))
        }
    }

    fn create_path(root: &Path, name: &PackageName, version: &PackageVersion) -> Result<PathBuf> {
        if !root.is_dir() {
            return Err(anyhow!("Cannot create file path for {}-{} when root is file path: {}",
                    name, version, root.display()))
        }

        Ok(root.join(format!("{}-{}", name, version)))
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::tests::pname;
    use crate::package::tests::pversion;

    #[test]
    fn test_parser_one_letter_name() {
        let p = PathBuf::from("a-1.ext");
        let root = PathBuf::from("/");
        let r = Artifact::parse_path(&root, &p);

        assert!(r.is_ok(), "Expected to be Ok(_): {:?}", r);
        let (name, version) = r.unwrap();

        assert_eq!(name, pname("a"));
        assert_eq!(version, pversion("1"));
    }

    #[test]
    fn test_parser_multi_letter_name() {
        let p = PathBuf::from("foo-1.ext");
        let root = PathBuf::from("/");
        let r = Artifact::parse_path(&root, &p);

        assert!(r.is_ok(), "Expected to be Ok(_): {:?}", r);
        let (name, version) = r.unwrap();

        assert_eq!(name, pname("foo"));
        assert_eq!(version, pversion("1"));
    }

    #[test]
    fn test_parser_multi_char_version() {
        let p = PathBuf::from("foo-1123.ext");
        let root = PathBuf::from("/");
        let r = Artifact::parse_path(&root, &p);

        assert!(r.is_ok(), "Expected to be Ok(_): {:?}", r);
        let (name, version) = r.unwrap();

        assert_eq!(name, pname("foo"));
        assert_eq!(version, pversion("1123"));
    }

    #[test]
    fn test_parser_multi_char_version_dashed() {
        let p = PathBuf::from("foo-1-1-2-3.ext");
        let root = PathBuf::from("/");
        let r = Artifact::parse_path(&root, &p);

        assert!(r.is_ok(), "Expected to be Ok(_): {:?}", r);
        let (name, version) = r.unwrap();

        assert_eq!(name, pname("foo"));
        assert_eq!(version, pversion("1-1-2-3"));
    }

    #[test]
    fn test_parser_multi_char_version_dashed_and_dotted() {
        let p = PathBuf::from("foo-1-1.2-3.ext");
        let root = PathBuf::from("/");
        let r = Artifact::parse_path(&root, &p);

        assert!(r.is_ok(), "Expected to be Ok(_): {:?}", r);
        let (name, version) = r.unwrap();

        assert_eq!(name, pname("foo"));
        assert_eq!(version, pversion("1-1.2-3"));
    }

    #[test]
    fn test_parser_alnum_version() {
        let p = PathBuf::from("foo-1-1.2a3.ext");
        let root = PathBuf::from("/");
        let r = Artifact::parse_path(&root, &p);

        assert!(r.is_ok(), "Expected to be Ok(_): {:?}", r);
        let (name, version) = r.unwrap();

        assert_eq!(name, pname("foo"));
        assert_eq!(version, pversion("1-1.2a3"));
    }

    #[test]
    fn test_parser_package_name_with_number() {
        let p = PathBuf::from("foo2-1-1.2a3.ext");
        let root = PathBuf::from("/");
        let r = Artifact::parse_path(&root, &p);

        assert!(r.is_ok(), "Expected to be Ok(_): {:?}", r);
        let (name, version) = r.unwrap();

        assert_eq!(name, pname("foo2"));
        assert_eq!(version, pversion("1-1.2a3"));
    }
}
