//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use resiter::AndThen;
use resiter::Filter;
use resiter::Map;
use tracing::trace;
use walkdir::DirEntry;
use walkdir::WalkDir;

use crate::repository::fs::element::Element;
use crate::repository::fs::path::PathComponent;

/// A type representing the filesystem
///
/// This type can be used to load pkg.toml files from the filesystem. As soon as this object is
/// loaded, all filesystem access is done and postprocessing of the loaded data can happen.
#[derive(Debug, getset::Getters)]
pub struct FileSystemRepresentation {
    #[getset(get = "pub")]
    root: PathBuf,

    #[getset(get = "pub")]
    files: Vec<PathBuf>,

    // A recursive data structure that represents the repository from the root.
    // Valid entries are:
    // - PkgToml -> File(content: String)
    // - DirName(name: String) -> Dir(entries: HashMap<PathComponent, Element>)
    // Directories are recursive and can contain (sub)directories (0-n) and at most one `pkg.toml`
    // file.
    elements: HashMap<PathComponent, Element>,
}

impl FileSystemRepresentation {
    /// Load the FileSystemRepresentation object starting at `root`.
    pub fn load(root: PathBuf) -> Result<Self> {
        let mut fsr = FileSystemRepresentation {
            root: root.clone(),
            elements: HashMap::new(),
            files: vec![],
        };

        // get the number of maximum files open (ulimit -n on Linux)
        let max_files_open = {
            let (soft, _hard) = rlimit::getrlimit(rlimit::Resource::NOFILE)?;

            // use less than the soft limit if the soft limit is above 15
            soft.checked_sub(16)
                .unwrap_or(soft)
                .try_into() // we need to have a usize
                .unwrap_or(usize::MAX) // if usize is smaller than u64, usize::MAX will do
        };

        trace!(
            "Loading files from filesystem starting at: {}",
            root.display()
        );
        trace!("Loading with a maximum of {} files open", max_files_open);
        WalkDir::new(root)
            .follow_links(false)
            .max_open(max_files_open)
            .same_file_system(true)
            .into_iter()
            .filter_entry(|e| !is_hidden(e) && (is_pkgtoml(e) || is_dir(e)))
            .filter_ok(is_pkgtoml)
            .inspect(|el| trace!("Loading: {:?}", el))
            .map_err(Error::from)
            .and_then_ok(|de| {
                let mut curr_hm = &mut fsr.elements;
                let de_path = de.path();
                fsr.files.push(de_path.to_path_buf());

                // Build/extend the HashMap tree by adding the current path (we strip the repo root
                // prefix since we're only interested in the structure of the repo below its root):
                let root_relative_path = de_path.strip_prefix(&fsr.root)?;
                for cmp in root_relative_path.components() {
                    match PathComponent::try_from(&cmp)? {
                        PathComponent::PkgToml => {
                            curr_hm
                                .entry(PathComponent::PkgToml)
                                .or_insert(Element::File(load_file(de_path)?));
                        }
                        dir @ PathComponent::DirName(_) => {
                            curr_hm
                                .entry(dir.clone())
                                .or_insert_with(|| Element::Dir(HashMap::new()));

                            // Step into the sub HashMap tree for the next iteration:
                            curr_hm = curr_hm
                                .get_mut(&dir)
                                .unwrap() // safe, because we just inserted it
                                .get_map_mut()
                                .unwrap(); // safe, because we inserted Element::Dir
                        }
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
    ///     /pkg.toml <-- is not a leaf
    ///     /foo/
    ///     /foo/pkg.toml <-- is leaf
    ///     /bar/
    ///     /bar/pkg.toml <-- is not a leaf
    ///     /bar/baz/pkg.toml <-- is a leaf
    ///
    ///
    pub fn is_leaf_file(&self, path: &Path) -> Result<bool> {
        // Helper to check whether a tree contains pkg.toml files, recursively
        fn toml_files_in_tree(hm: &HashMap<PathComponent, Element>) -> bool {
            // This is an optional optimization (should save time by avoiding potentially
            // unnecessarily recursing the tree at the cost of this extra check) since we also
            // check for this case in the match below (i.e., it's a shortcut that doesn't affect
            // correctness):
            if hm.contains_key(&PathComponent::PkgToml) {
                return true;
            }

            for value in hm.values() {
                match value {
                    Element::File(_) => return true,
                    Element::Dir(hm) => {
                        if toml_files_in_tree(hm) {
                            return true;
                        }
                    }
                }
            }
            false
        }

        // Traverse the repo tree via a root relative path and check if the current tree
        // (directory) contains other `pkg.toml` files once we've hit a `pkg.toml` file:
        let path = path.strip_prefix(&self.root).with_context(|| {
            anyhow!(
                "The path `{}` doesn't include the repo root `{}`",
                path.display(),
                &self.root.display()
            )
        })?;
        let mut curr_hm = &self.elements;
        for elem in path.components() {
            let elem = PathComponent::try_from(&elem)?;

            match curr_hm.get(&elem) {
                Some(Element::File(_)) => {
                    // We've hit a `pkg.toml` file (should be the final iteration) and it is a leaf
                    // file, if the current hashmap only holds either:
                    // * No directory (i.e., just one entry: this `pkg.toml` file)
                    // * Directories where all subdirs (recursive) do not contain a `pkg.toml` file
                    return Ok(curr_hm.values().count() == 1 || !toml_files_in_tree(curr_hm));
                }
                Some(Element::Dir(hm)) => curr_hm = hm, // Move into the subtree
                None => anyhow::bail!(
                    "Path component '{:?}' was not loaded in map, this is most likely a bug",
                    elem
                ),
            }
        }

        Ok(false)
    }

    /// Get a Vec<(PathBuf, &String)> for the `path`.
    ///
    /// The result of this function is the trail of `pkg.toml` files from `self.root` to `path`,
    /// whereas the PathBuf is the actual path to the file and the `&String` is the content of the
    /// individual file.
    ///
    /// Merging all Strings in the returned Vec as Config objects should produce a Package.
    pub fn get_files_for<'a>(&'a self, path: &Path) -> Result<Vec<(PathBuf, &'a String)>> {
        let mut res = Vec::with_capacity(10); // good enough

        // Traverse the repo tree via a root relative path and collect all `pkg.toml` files along
        // the way (we'll include self.root in the returned paths):
        let path = path.strip_prefix(&self.root).with_context(|| {
            anyhow!(
                "The path `{}` doesn't include the repo root `{}`",
                path.display(),
                &self.root.display()
            )
        })?;
        let mut curr_hm = &self.elements;
        let mut curr_path = self.root.clone();
        for elem in path.components() {
            let elem = PathComponent::try_from(&elem)?;

            match curr_hm.get(&elem) {
                Some(Element::File(cont)) => res.push((curr_path.join("pkg.toml"), cont)),
                Some(Element::Dir(hm)) => {
                    if let Some(Element::File(intermediate)) = curr_hm.get(&PathComponent::PkgToml)
                    {
                        // The current directory contains a `pkg.toml` file -> add it:
                        res.push((curr_path.join("pkg.toml"), intermediate));
                    }
                    // Move into the directory/subtree:
                    curr_path = curr_path.join(elem.dir_name().unwrap()); // unwrap safe by above match
                    curr_hm = hm;
                }
                None => anyhow::bail!(
                    "Path component '{:?}' was not loaded in map, this is most likely a bug",
                    elem
                ),
            }
        }

        Ok(res)
    }
}

/// Helper to check whether a DirEntry points to a hidden file
fn is_hidden(entry: &DirEntry) -> bool {
    trace!("Check {:?} is hidden", entry);
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

/// Helper to check whether a DirEntry points to a directory
fn is_dir(entry: &DirEntry) -> bool {
    trace!("Check {:?} is directory", entry);
    entry.file_type().is_dir()
}

/// Helper to check whether a DirEntry points to a pkg.toml file
fn is_pkgtoml(entry: &DirEntry) -> bool {
    trace!("Check {:?} == 'pkg.toml'", entry);
    entry
        .file_name()
        .to_str()
        .map(|s| s == "pkg.toml")
        .unwrap_or(false)
}

/// Helper fn to load a Path into memory as String
fn load_file(path: &Path) -> Result<String> {
    trace!("Reading {}", path.display());
    std::fs::read_to_string(path)
        .with_context(|| anyhow!("Reading file from filesystem: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str, hm: Vec<(PathComponent, Element)>) -> (PathComponent, Element) {
        (
            PathComponent::DirName(name.to_string()),
            Element::Dir(hm.into_iter().collect()),
        )
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
            //  /foo/pkg.toml: content
            elements: vec![dir("foo", vec![pkgtoml("content")])]
                .into_iter()
                .collect(),

            files: vec![PathBuf::from("/foo/pkg.toml")],
        };

        let path = "/foo/pkg.toml".as_ref();

        assert!(fsr.is_leaf_file(path).unwrap());
        assert_eq!(
            fsr.get_files_for(path).unwrap(),
            vec![(pb("/foo/pkg.toml"), &s("content"))]
        );
    }

    #[test]
    fn test_deep_pkgtoml() {
        let fsr = FileSystemRepresentation {
            root: PathBuf::from("/"),

            // Representing
            //  /
            //  /foo
            //  /foo/bar
            //  /foo/bar/baz
            //  /foo/bar/baz/pkg.toml: content
            elements: vec![dir(
                "foo",
                vec![dir("bar", vec![dir("baz", vec![pkgtoml("content")])])],
            )]
            .into_iter()
            .collect(),

            files: vec![PathBuf::from("/foo/bar/baz/pkg.toml")],
        };

        let path = "/foo/bar/baz/pkg.toml".as_ref();

        assert!(fsr.is_leaf_file(path).unwrap());
        assert_eq!(
            fsr.get_files_for(path).unwrap(),
            vec![(pb("/foo/bar/baz/pkg.toml"), &s("content"))]
        );
    }

    #[test]
    fn test_hierarchy() {
        let fsr = FileSystemRepresentation {
            root: PathBuf::from("/"),

            // Representing
            //  /
            //  /foo
            //  /foo/pkg.toml: content1
            //  /foo/bar
            //  /foo/bar/pkg.toml: content2
            //  /foo/bar/baz
            //  /foo/bar/baz/pkg.toml: content3
            elements: vec![dir(
                "foo",
                vec![
                    pkgtoml("content1"),
                    dir(
                        "bar",
                        vec![pkgtoml("content2"), dir("baz", vec![pkgtoml("content3")])],
                    ),
                ],
            )]
            .into_iter()
            .collect(),

            files: vec![
                PathBuf::from("/foo/pkg.toml"),
                PathBuf::from("/foo/bar/pkg.toml"),
                PathBuf::from("/foo/bar/baz/pkg.toml"),
            ],
        };

        {
            let path = "/foo/pkg.toml".as_ref();

            assert!(!fsr.is_leaf_file(path).unwrap());
        }
        {
            let path = "/foo/bar/pkg.toml".as_ref();

            assert!(!fsr.is_leaf_file(path).unwrap());
        }
        {
            let path = "/foo/bar/baz/pkg.toml".as_ref();

            assert!(fsr.is_leaf_file(path).unwrap());
            assert_eq!(
                fsr.get_files_for(path).unwrap(),
                vec![
                    (pb("/foo/pkg.toml"), &s("content1")),
                    (pb("/foo/bar/pkg.toml"), &s("content2")),
                    (pb("/foo/bar/baz/pkg.toml"), &s("content3")),
                ]
            );
        }
    }

    #[test]
    fn test_hierarchy_with_missing_intermediate_files() {
        let fsr = FileSystemRepresentation {
            root: PathBuf::from("/"),

            // Representing
            //  /
            //  /foo
            //  /foo/pkg.toml: content1
            //  /foo/bar
            //  /foo/bar/baz
            //  /foo/bar/baz/pkg.toml: content3
            elements: vec![dir(
                "foo",
                vec![
                    pkgtoml("content1"),
                    dir("bar", vec![dir("baz", vec![pkgtoml("content3")])]),
                ],
            )]
            .into_iter()
            .collect(),

            files: vec![
                PathBuf::from("/foo/pkg.toml"),
                PathBuf::from("/foo/bar/baz/pkg.toml"),
            ],
        };

        let path = "/foo/pkg.toml".as_ref();
        assert!(!fsr.is_leaf_file(path).unwrap());

        let path = "/foo/bar/baz/pkg.toml".as_ref();
        assert!(fsr.is_leaf_file(path).unwrap());
        assert_eq!(
            fsr.get_files_for(path).unwrap(),
            vec![
                (pb("/foo/pkg.toml"), &s("content1")),
                (pb("/foo/bar/baz/pkg.toml"), &s("content3")),
            ]
        );
    }

    #[test]
    fn test_hierarchy_with_toplevel_file() {
        let fsr = FileSystemRepresentation {
            root: PathBuf::from("/"),

            // Representing
            //  /
            //  /pkg.toml: content1
            //  /foo
            //  /foo/bar
            //  /foo/bar/baz
            //  /foo/bar/baz/pkg.toml: content3
            elements: vec![
                pkgtoml("content1"),
                dir(
                    "foo",
                    vec![dir("bar", vec![dir("baz", vec![pkgtoml("content3")])])],
                ),
            ]
            .into_iter()
            .collect(),

            files: vec![
                PathBuf::from("/pkg.toml"),
                PathBuf::from("/foo/bar/baz/pkg.toml"),
            ],
        };

        let path = "/pkg.toml".as_ref();
        assert!(!fsr.is_leaf_file(path).unwrap());

        let path = "/foo/bar/baz/pkg.toml".as_ref();
        assert!(fsr.is_leaf_file(path).unwrap());
        assert_eq!(
            fsr.get_files_for(path).unwrap(),
            vec![
                (pb("/pkg.toml"), &s("content1")),
                (pb("/foo/bar/baz/pkg.toml"), &s("content3")),
            ]
        );
    }

    #[test]
    fn test_loading_the_example_repo() -> Result<()> {
        fn pb(repo_relative_path: &str) -> PathBuf {
            PathBuf::from("examples/packages/repo/").join(repo_relative_path)
        }
        fn ps(repo_relative_path: &str) -> String {
            String::from(pb(repo_relative_path).to_string_lossy())
        }

        let fsr = FileSystemRepresentation::load(pb(""))?;

        // Test the leaf file logic:
        assert!(!fsr.is_leaf_file(&pb("pkg.toml")).unwrap());
        assert!(!fsr.is_leaf_file(&pb("s/pkg.toml")).unwrap());
        assert!(fsr.is_leaf_file(&pb("s/19.0/pkg.toml")).unwrap());
        assert!(fsr.is_leaf_file(&pb("invalid/pkg.toml")).is_err());

        // Test if all pkg.toml files get found/loaded and check the leaf files count:
        let pkgtoml_files_count = 31; // find examples/packages/repo/ -name pkg.toml | wc -l
        assert_eq!(fsr.files().len(), pkgtoml_files_count);
        // Manually count the non-leaf files:
        let non_leaf_files_count = 2;
        // Or the following can be used to count the leaf directories (with quite a few asterisks though!):
        // find examples/packages/repo/ -type d -links 2 | wc -l
        let leaf_files_count = fsr
            .files()
            .iter()
            .filter(|path| fsr.is_leaf_file(path).unwrap())
            .count();
        assert_eq!(leaf_files_count, pkgtoml_files_count - non_leaf_files_count);

        // Test getting all pkg.toml files for a given path:
        assert!(fsr.get_files_for(&pb("invalid/pkg.toml")).is_err());
        fn assert_files_for(fsr: &FileSystemRepresentation, path: &str, files: Vec<&PathBuf>) {
            assert_eq!(
                fsr.get_files_for(path.as_ref())
                    .unwrap()
                    .iter()
                    .map(|(path, _)| path)
                    .collect::<Vec<_>>(),
                files,
                "Get files for {path}"
            );
        }
        assert_files_for(&fsr, &ps("pkg.toml"), vec![&pb("pkg.toml")]);
        assert_files_for(
            &fsr,
            &ps("z/pkg.toml"),
            vec![&pb("pkg.toml"), &pb("z/pkg.toml")],
        );
        assert_files_for(
            &fsr,
            &ps("s/19.1/pkg.toml"),
            vec![&pb("pkg.toml"), &pb("s/pkg.toml"), &pb("s/19.1/pkg.toml")],
        );

        Ok(())
    }
}
