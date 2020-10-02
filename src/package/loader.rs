use std::path::PathBuf;
use std::path::Path;
use std::ops::Deref;
use anyhow::Result;
use anyhow::Error;
use resiter::FilterMap;
use resiter::Map;
use resiter::AndThen;
use resiter::Filter;
use walkdir::WalkDir;

use crate::package::Package;
use crate::package::PackageName;
use crate::package::PackageVersion;
use crate::package::PackageVersionConstraint;

pub struct Loader {
    root: PathBuf,
}

impl Loader {
    pub fn new(root_path: PathBuf) -> Self {
        Loader { root: root_path }
    }

    pub fn load<PN, PVC>(&self, package_name: PN, package_version: PVC) -> Result<Option<Package>>
        where PN: AsRef<PackageName>,
            PVC: AsRef<PackageVersionConstraint>,
    {
        find_in(&self.root, config::Config::default(), package_name.as_ref(), package_version.as_ref())
            .and_then(|o| if let Some(config) = o {
                config.deserialize().map_err(Error::from)
            } else {
                Ok(None)
            })
    }

}

/// Finds the most specific package file for a package with the name `package_name` and the
/// version `package_version`
///
/// # Problem description
///
/// Assume the following filesystem tree:
///
///     /
///     /pkg
///     /vim/
///     /vim/pkg
///     /vim/8.0/pkg
///     /vim/8.1/pkg
///     /vim/8.2/pkg
///     /htop/
///     /htop/pkg
///     /htop/2.2.0/pkg
///     /htop/2.3.0/pkg
///     /htop/3.0.0/pkg
///
/// Note that the name of a directory is _NOT_ relevant to this function.
/// All information that is aquired during the execution of this function comes from the "pkg"
/// files.
///
/// If `find_package_file("vim", "8.1") is now called, the function starts traversing the
/// filesystem at the root directory.
/// it finds the "pkg" file in `/vim/pkg` which contains _only_ the information that this is
/// the "vim" package definition sub-tree.
/// The function can from then on ignore `/htop` and other directories.
/// The function then may iterate down into the subdirectories of the `/vim` subtree and parse
/// all `pkg` files in there, which ultimatively yield the desired version of vim in one of the
/// files.
/// This is then the file(path) which should be returned.
///
/// The function, although, can _not_ just check the files at the deepest level, as these files
/// may not contain the package name at all, just the version (and possibly some other
/// information).
///
///
/// # Implementation
///
/// The problem is solved recursively.
///
/// 1. parse the `pkg` file at the current recursion level
/// 1. do we have the right package name and a constraint-satisfying package version?
///     1. yes: return
///     1. no: for each subdir at current level, recurse. If there is a satisfying package,
///        return it
///
fn find_in(path: &Path, config: config::Config, package_name: &PackageName, package_version: &PackageVersionConstraint)
    -> Result<Option<config::Config>>
{
    let pkg_file = path.join("pkg.toml");
    let mut c    = config.clone();

    if pkg_file.is_file() {
        let buf  = std::fs::read_to_string(&pkg_file)?;
        let file = config::File::from_str(&buf, config::FileFormat::Toml);
        c.merge(file)?;
    }

    if name_match(&c, package_name) {
        for subdir in all_subdirs(path)? {
            match find_in(&subdir, c.clone(), package_name, package_version) {
                Ok(Some(cfg)) => {
                    if version_match(&cfg, package_version)? {
                        return Ok(Some(cfg))
                    } else {
                        continue
                    }
                },
                Ok(None) => continue,
                Err(e) => return Err(e),
            }
        }

        Ok(Some(c))
    } else {
        Ok(None)
    }
}

fn name_match(config: &config::Config, package_name: &PackageName) -> bool {
    match config.get_str("name") {
        Ok(s) => s == *package_name.deref(),
        Err(_) => false,
    }
}

fn version_match(config: &config::Config, package_version: &PackageVersionConstraint)
    -> Result<bool>
{
    match config.get_str("version") {
        Ok(s) => Ok(!package_version.matches(&PackageVersion::from(s))?.is_false()),
        Err(_) => Ok(false),
    }
}

fn all_subdirs(p: &Path) -> Result<Vec<PathBuf>> {
    let mut v = Vec::new();
    for de in p.read_dir()? {
        let de = de?;
        if de.file_type()?.is_dir() {
            v.push(de.path());
        }
    }

    return Ok(v)
}
