//! Utility functions for the UI

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use handlebars::Handlebars;
use itertools::Itertools;

use crate::package::Package;

pub fn package_repo_cleanness_check(repo_path: &Path) -> Result<()> {
    if !crate::util::git::repo_is_clean(&repo_path)? {
        error!("Repository not clean, refusing to go on: {}", repo_path.display());
        Err(anyhow!("Repository not clean, refusing to go on: {}", repo_path.display()))
    } else {
        Ok(())
    }
}

pub fn print_packages<'a, I>(out: &mut dyn Write,
                             format: &str,
                             iter: I,
                             print_runtime_deps: bool,
                             print_build_deps: bool,
                             print_sys_deps: bool,
                             print_sys_runtime_deps: bool)
-> Result<()>
    where I: Iterator<Item = &'a Package>
{
    let mut hb = Handlebars::new();
    hb.register_template_string("package", format)?;

    for (i, package) in iter.enumerate() {
        print_package(out,
                      &hb,
                      i,
                      package,
                      print_runtime_deps,
                      print_build_deps,
                      print_sys_deps,
                      print_sys_runtime_deps
                      )?;
    }

    Ok(())
}

fn print_package(out: &mut dyn Write,
                 hb: &Handlebars,
                 i: usize,
                 package: &Package,
                 print_runtime_deps: bool,
                 print_build_deps: bool,
                 print_sys_deps: bool,
                 print_sys_runtime_deps: bool
                 )
    -> Result<()>
{
    let mut data = BTreeMap::new();
    data.insert("i", i.to_string());
    data.insert("name",             format!("{}", package.name()));
    data.insert("version",          format!("{}", package.version()));
    data.insert("source_url",       format!("{}", package.source().url()));
    data.insert("source_hash_type", format!("{}", package.source().hash().hashtype()));
    data.insert("source_hash",      format!("{}", package.source().hash().value()));

    // This is an ugly hack. Because the `data` is a <String, String>, we do only insert the flag
    // if it is set, because handlebars renders a non-present value as false
    if print_runtime_deps {
        data.insert("print_runtime_deps",     format!("{}", print_runtime_deps));
    }
    if print_build_deps {
        data.insert("print_build_deps",       format!("{}", print_build_deps));
    }
    if print_sys_deps {
        data.insert("print_system_deps",         format!("{}", print_sys_deps));
    }
    if print_sys_runtime_deps {
        data.insert("print_system_runtime_deps", format!("{}", print_sys_runtime_deps));
    }

    data.insert("runtime_deps",     {
        format!("[{}]", package.dependencies()
                .runtime()
                .iter()
                .map(|p| p.as_ref())
                .join(", "))
    });
    data.insert("build_deps",     {
        format!("[{}]", package.dependencies()
                .build()
                .iter()
                .map(|p| p.as_ref())
                .join(", "))
    });
    data.insert("system_deps",     {
        format!("[{}]", package.dependencies()
                .system()
                .iter()
                .map(|p| p.as_ref())
                .join(", "))
    });
    data.insert("system_runtime_deps",     {
        format!("[{}]", package.dependencies()
                .system_runtime()
                .iter()
                .map(|p| p.as_ref())
                .join(", "))
    });

    hb.render("package", &data)
        .map_err(Error::from)
        .and_then(|r| writeln!(out, "{}", r).map_err(Error::from))
}

