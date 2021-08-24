#![allow(unused)] // TODO: Remove allow(unused)

use std::path::Path;
use std::path::PathBuf;
use std::collections::HashMap;
use std::path::Component;
use std::convert::TryFrom;

use walkdir::DirEntry;
use walkdir::WalkDir;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use anyhow::Error;
use resiter::Map;
use resiter::AndThen;

#[derive(Debug)]
pub struct FileSystemRepresentation {
    root: PathBuf,
    elements: HashMap<PathComponent, Element>,
    files: Vec<PathBuf>,
}

#[derive(Debug)]
enum Element {
    File(String),
    Dir(HashMap<PathComponent, Element>)
}

impl Element {
    fn get_map_mut(&mut self) -> Option<&mut HashMap<PathComponent, Element>> {
        match self {
            Element::File(_) => None,
            Element::Dir(ref mut hm) => Some(hm),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PathComponent {
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
            },
        }
    }
}

impl PathComponent {
    fn is_pkg_toml(&self) -> bool {
        std::matches!(self, PathComponent::PkgToml)
    }
}


impl FileSystemRepresentation {
    pub fn load(root: PathBuf) -> Result<Self> {
        let mut fsr = FileSystemRepresentation {
            root: root.clone(),
            elements: HashMap::new(),
            files: vec![],
        };

        WalkDir::new(root)
            .follow_links(false)
            .max_open(100)
            .same_file_system(true)
            .into_iter()
            .filter_entry(|e| is_pkgtoml(e))
            .inspect(|el| log::trace!("Loading: {:?}", el))
            .map_err(Error::from)
            .and_then_ok(|de| {
                let mut curr_hm = &mut fsr.elements;
                fsr.files.push(de.path().to_path_buf());

                // traverse the HashMap tree
                for cmp in de.path().components() {
                    match PathComponent::try_from(&cmp)? {
                        PathComponent::PkgToml => {
                            curr_hm.entry(PathComponent::PkgToml)
                                .or_insert(Element::File(load_file(de.path())?));
                        },
                        dir @ PathComponent::DirName(_) => {
                            curr_hm.entry(dir.clone())
                                .or_insert(Element::Dir(HashMap::new()));

                            curr_hm = curr_hm.get_mut(&dir)
                                .unwrap() // safe, because we just inserted it
                                .get_map_mut()
                                .unwrap(); // safe, because we inserted Element::Dir
                        },
                    }
                }

                Ok(())
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(fsr)
    }

    pub fn is_leaf_file(&self, path: &Path) -> Result<bool> {
        let mut curr_hm = &self.elements;

        fn toml_files_in_tree(hm: &HashMap<PathComponent, Element>) -> bool {
            for value in hm.values() {
                match value {
                    Element::File(_) => return true,
                    Element::Dir(hm) => if toml_files_in_tree(hm) {
                        return true
                    },
                }
            }
            false
        }

        for elem in path.components() {
            let elem = PathComponent::try_from(&elem)?;

            match curr_hm.get(&elem) {
                Some(Element::File(_)) => {
                    // if I have a file now, and the current hashmap only holds either
                    // * No directory
                    // * or a directory where all subdirs do not contain a pkg.toml
                    return Ok(curr_hm.values().count() == 1 || !toml_files_in_tree(curr_hm))
                },
                Some(Element::Dir(hm)) => curr_hm = hm,
                None => {
                    unimplemented!()
                },
            }
        }

        Ok(false)
    }

    pub fn get_files_for<'a>(&'a self, path: &Path) -> Result<Vec<&'a String>> {
        let mut res = Vec::with_capacity(10); // good enough

        let mut curr_hm = &self.elements;
        for elem in path.components() {
            let elem = PathComponent::try_from(&elem)?;

            if !elem.is_pkg_toml() {
                if let Some(Element::File(intermediate)) = curr_hm.get(&PathComponent::PkgToml) {
                    res.push(intermediate);
                }
            }

            match curr_hm.get(&elem) {
                Some(Element::File(cont)) => res.push(cont),
                Some(Element::Dir(hm)) => curr_hm = hm,
                None => {
                    unimplemented!()
                },
            }
        }

        Ok(res)
    }
}

fn is_pkgtoml(entry: &DirEntry) -> bool {
    entry.file_name().to_str().map(|s| s == "pkg.toml").unwrap_or(false)
}

fn load_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| anyhow!("Reading file from filesystem: {}", path.display()))
        .map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str, hm: Vec<(PathComponent, Element)>) -> (PathComponent, Element) {
        (PathComponent::DirName(name.to_string()), Element::Dir(hm.into_iter().collect()))
    }

    fn pkgtoml(content: &str) -> (PathComponent, Element) {
        (PathComponent::PkgToml, Element::File(content.to_string()))
    }

    #[test]
    fn test_one_file_in_directory() {
        let fsr = FileSystemRepresentation {
            root: PathBuf::from("/"),

            // Representing
            //  /
            //  /foo
            //  /foo/pkg.toml
            elements: vec![
                dir("foo", vec![
                    pkgtoml("content")
                ])
            ].into_iter().collect(),

            files: vec![
                PathBuf::from("foo/pkg.toml")
            ],
        };

        let path = "foo/pkg.toml".as_ref();

        assert!(fsr.is_leaf_file(path).unwrap());
        assert_eq!(fsr.get_files_for(path).unwrap(), vec!["content"]);
    }

    #[test]
    fn test_deep_pkgtoml() {
        let fsr = FileSystemRepresentation {
            root: PathBuf::from("/"),

            // Representing
            //  /
            //  /foo
            //  /foo/bar
            //  /foo/baz
            //  /foo/baz/pkg.toml
            elements: vec![
                dir("foo", vec![
                    dir("bar", vec![
                        dir("baz", vec![
                            pkgtoml("content"),
                        ])
                    ])
                ])
            ].into_iter().collect(),

            files: vec![
                PathBuf::from("foo/bar/baz/pkg.toml")
            ],
        };

        let path = "foo/bar/baz/pkg.toml".as_ref();

        assert!(fsr.is_leaf_file(path).unwrap());
        assert_eq!(fsr.get_files_for(path).unwrap(), vec!["content"]);
    }

    #[test]
    fn test_hierarchy() {
        let fsr = FileSystemRepresentation {
            root: PathBuf::from("/"),

            // Representing
            //  /
            //  /foo
            //  /foo/bar
            //  /foo/baz
            //  /foo/baz/pkg.toml
            elements: vec![
                dir("foo", vec![
                    pkgtoml("content1"),
                    dir("bar", vec![
                        pkgtoml("content2"),
                        dir("baz", vec![
                            pkgtoml("content3"),
                        ])
                    ])
                ])
            ].into_iter().collect(),

            files: vec![
                PathBuf::from("foo/pkg.toml"),
                PathBuf::from("foo/bar/pkg.toml"),
                PathBuf::from("foo/bar/baz/pkg.toml")
            ],
        };

        {
            let path = "foo/pkg.toml".as_ref();

            assert!(!fsr.is_leaf_file(path).unwrap());
        }
        {
            let path = "foo/bar/pkg.toml".as_ref();

            assert!(!fsr.is_leaf_file(path).unwrap());
        }
        {
            let path = "foo/bar/baz/pkg.toml".as_ref();

            assert!(fsr.is_leaf_file(path).unwrap());
            assert_eq!(fsr.get_files_for(path).unwrap(), vec!["content1", "content2", "content3"]);
        }
    }

    #[test]
    fn test_hierarchy_with_missing_intermediate_files() {
        let fsr = FileSystemRepresentation {
            root: PathBuf::from("/"),

            // Representing
            //  /
            //  /foo
            //  /foo/bar
            //  /foo/baz
            //  /foo/baz/pkg.toml
            elements: vec![
                dir("foo", vec![
                    pkgtoml("content1"),
                    dir("bar", vec![
                        dir("baz", vec![
                            pkgtoml("content3"),
                        ])
                    ])
                ])
            ].into_iter().collect(),

            files: vec![
                PathBuf::from("foo/pkg.toml"),
                PathBuf::from("foo/bar/baz/pkg.toml")
            ],
        };

        let path = "foo/pkg.toml".as_ref();
        assert!(!fsr.is_leaf_file(path).unwrap());

        let path = "foo/bar/baz/pkg.toml".as_ref();
        assert!(fsr.is_leaf_file(path).unwrap());
        assert_eq!(fsr.get_files_for(path).unwrap(), vec!["content1", "content3"]);
    }

    #[test]
    fn test_hierarchy_with_toplevel_file() {
        let fsr = FileSystemRepresentation {
            root: PathBuf::from("/"),

            // Representing
            //  /
            //  /foo
            //  /foo/bar
            //  /foo/baz
            //  /foo/baz/pkg.toml
            elements: vec![
                pkgtoml("content1"),
                dir("foo", vec![
                    dir("bar", vec![
                        dir("baz", vec![
                            pkgtoml("content3"),
                        ])
                    ])
                ])
            ].into_iter().collect(),

            files: vec![
                PathBuf::from("pkg.toml"),
                PathBuf::from("foo/bar/baz/pkg.toml")
            ],
        };

        let path = "pkg.toml".as_ref();
        assert!(!fsr.is_leaf_file(path).unwrap());

        let path = "foo/bar/baz/pkg.toml".as_ref();
        assert!(fsr.is_leaf_file(path).unwrap());
        assert_eq!(fsr.get_files_for(path).unwrap(), vec!["content1", "content3"]);
    }

}
