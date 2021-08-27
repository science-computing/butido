#![allow(unused)] // TODO: Remove allow(unused)

use std::path::Path;
use std::path::PathBuf;
use std::collections::HashMap;
use std::path::Component;
use std::convert::TryFrom;
use std::convert::TryInto;

use walkdir::DirEntry;
use walkdir::WalkDir;
use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use anyhow::Error;
use resiter::Filter;
use resiter::Map;
use resiter::AndThen;

/// A type representing the filesystem
///
/// This type can be used to load pkg.toml files from the filesystem. As soon as this object is
/// loaded, all filesystem access is done and postprocessing of the loaded data can happen
#[derive(Debug, getset::Getters)]
pub struct FileSystemRepresentation {
    #[getset(get = "pub")]
    root: PathBuf,

    #[getset(get = "pub")]
    files: Vec<PathBuf>,

    elements: HashMap<PathComponent, Element>,
}

/// One element in the tree inside FileSystemRepresentation
///
/// This is either a File, or a Directory that contains more (Files or Directories).
#[derive(Debug)]
enum Element {
    File(String),
    Dir(HashMap<PathComponent, Element>)
}

impl Element {
    /// Helper fn to get the directory contents of the element, if the element is an Element::Dir
    fn get_map_mut(&mut self) -> Option<&mut HashMap<PathComponent, Element>> {
        match self {
            Element::File(_) => None,
            Element::Dir(ref mut hm) => Some(hm),
        }
    }
}

/// Helper type for filtering for pathes we need or dont need
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
    /// Helper fn whether this PathComponent is a PathComponent::PkgToml
    fn is_pkg_toml(&self) -> bool {
        std::matches!(self, PathComponent::PkgToml)
    }

    /// Helper fn to get the directory name of this PathComponent if it is a PathComponent::DirName
    /// or None if it is not.
    fn dir_name(&self) -> Option<&str> {
        match self {
            PathComponent::PkgToml => None,
            PathComponent::DirName(dn) => Some(dn)
        }
    }
}


impl FileSystemRepresentation {
    /// Load the FileSystemRepresentation object starting a `root`.
    pub fn load(root: PathBuf) -> Result<Self> {
        let mut fsr = FileSystemRepresentation {
            root: root.clone(),
            elements: HashMap::new(),
            files: vec![],
        };

        // get the number of maximum files open (ulimit -n on linux)
        let max_files_open = {
            let (soft, _hard) = rlimit::getrlimit(rlimit::Resource::NOFILE)?;

            // use less than the soft limit if the soft limit is above 15
            soft.checked_sub(16)
                .unwrap_or(soft)
                .try_into() // we need to have a usize
                .unwrap_or(usize::MAX) // if usize is smaller than u64, usize::MAX will do
        };

        log::trace!("Loading files from filesystem starting at: {}", root.display());
        log::trace!("Loading with a maximum of {} files open", max_files_open);
        WalkDir::new(root)
            .follow_links(false)
            .max_open(max_files_open)
            .same_file_system(true)
            .into_iter()
            .filter_entry(|e| !is_hidden(e) && (is_pkgtoml(e) || is_dir(e)))
            .filter_ok(|e| is_pkgtoml(e))
            .inspect(|el| log::trace!("Loading: {:?}", el))
            .map_err(Error::from)
            .and_then_ok(|de| {
                let mut curr_hm = &mut fsr.elements;
                let de_path = de.path().strip_prefix(&fsr.root)?;
                fsr.files.push(de_path.to_path_buf());

                // traverse the HashMap tree
                for cmp in de_path.components() {
                    match PathComponent::try_from(&cmp)? {
                        PathComponent::PkgToml => {
                            curr_hm.entry(PathComponent::PkgToml)
                                .or_insert(Element::File(load_file(de_path)?));
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

    /// Check the tree whether a `Path` points to a file in a directory that does not contain more
    /// directories containing pkg.toml files.
    ///
    /// # Example
    ///
    ///     /
    ///     /foo/
    ///     /foo/pkg.toml <-- is leaf
    ///     /bar/
    ///     /bar/pkg.toml <-- is not a leaf
    ///     /bar/baz/pkg.toml <-- is a leaf
    ///
    ///
    pub fn is_leaf_file(&self, path: &Path) -> Result<bool> {
        let mut curr_hm = &self.elements;

        // Helper to check whether a tree contains pkg.toml files, recursively
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

    /// Get a Vec<(PathBuf, &String)> for the `path`
    ///
    /// The result of this function is the trail of pkg.toml files from `self.root` to `path`,
    /// whereas the PathBuf is the actual path to the file and the `&String` is the content of the
    /// individual file.
    ///
    /// Merging all Strings in the returned Vec as Config objects should produce a Package. to
    /// `path`, whereas the PathBuf is the actual path to the file and the `&String` is the content
    /// of the individual file.
    ///
    /// Merging all Strings in the returned Vec as Config objects should produce a Package.
    pub fn get_files_for<'a>(&'a self, path: &Path) -> Result<Vec<(PathBuf, &'a String)>> {
        let mut res = Vec::with_capacity(10); // good enough

        let mut curr_hm = &self.elements;
        let mut curr_path = PathBuf::from("");
        for elem in path.components() {
            let elem = PathComponent::try_from(&elem)?;

            if !elem.is_pkg_toml() {
                if let Some(Element::File(intermediate)) = curr_hm.get(&PathComponent::PkgToml) {
                    res.push((curr_path.join("pkg.toml"), intermediate));
                }
            }

            match curr_hm.get(&elem) {
                Some(Element::File(cont)) => res.push((curr_path.join("pkg.toml"), cont)),
                Some(Element::Dir(hm)) => {
                    curr_path = curr_path.join(elem.dir_name().unwrap()); // unwrap safe by above match
                    curr_hm = hm;
                }
                None => {
                    unimplemented!()
                },
            }
        }

        Ok(res)
    }
}

/// Helper to check whether a DirEntry points to a hidden file
fn is_hidden(entry: &DirEntry) -> bool {
    log::trace!("Check {:?} is hidden", entry);
    entry.file_name().to_str().map(|s| s.starts_with(".")).unwrap_or(false)
}

/// Helper to check whether a DirEntry points to a directory
fn is_dir(entry: &DirEntry) -> bool {
    log::trace!("Check {:?} is directory", entry);
    entry.file_type().is_dir()
}

/// Helper to check whether a DirEntry points to a pkg.toml file
fn is_pkgtoml(entry: &DirEntry) -> bool {
    log::trace!("Check {:?} == 'pkg.toml'", entry);
    entry.file_name().to_str().map(|s| s == "pkg.toml").unwrap_or(false)
}

/// Helper fn to load a Path into memory as String
fn load_file(path: &Path) -> Result<String> {
    log::trace!("Reading {}", path.display());
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

    fn pb(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    fn s(s: &str) -> String {
        String::from(s)
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
        assert_eq!(fsr.get_files_for(path).unwrap(), vec![(pb("foo/pkg.toml"), &s("content"))]);
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
        assert_eq!(fsr.get_files_for(path).unwrap(), vec![(pb("foo/bar/baz/pkg.toml"), &s("content"))]);
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
            assert_eq!(fsr.get_files_for(path).unwrap(), vec![
                (pb("foo/pkg.toml"),         &s("content1")),
                (pb("foo/bar/pkg.toml"),     &s("content2")),
                (pb("foo/bar/baz/pkg.toml"), &s("content3")),
            ]);
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
        assert_eq!(fsr.get_files_for(path).unwrap(), vec![
            (pb("foo/pkg.toml"),         &s("content1")),
            (pb("foo/bar/baz/pkg.toml"), &s("content3")),
        ]);
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
        assert_eq!(fsr.get_files_for(path).unwrap(), vec![
            (pb("pkg.toml"),             &s("content1")),
            (pb("foo/bar/baz/pkg.toml"), &s("content3")),
        ]);
    }

}
