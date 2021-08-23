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

pub struct FileSystemRepresentation {
    root: PathBuf,
    elements: HashMap<PathComponent, Element>,
    files: Vec<PathBuf>,
}

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

#[derive(Clone, PartialEq, Eq, Hash)]
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
}

fn is_pkgtoml(entry: &DirEntry) -> bool {
    entry.file_name().to_str().map(|s| s == "pkg.toml").unwrap_or(false)
}

fn load_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .with_context(|| anyhow!("Reading file from filesystem: {}", path.display()))
        .map_err(Error::from)
}

