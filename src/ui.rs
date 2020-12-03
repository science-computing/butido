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
                             print_build_deps: bool)
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
                 )
    -> Result<()>
{
    let mut data = BTreeMap::new();
    data.insert("i", serde_json::Value::Number(serde_json::Number::from(i)));
    data.insert("p", serde_json::to_value(package)?);

    // This is an ugly hack. Because the `data` is a <String, String>, we do only insert the flag
    // if it is set, because handlebars renders a non-present value as false
    if print_runtime_deps {
        data.insert("print_runtime_deps", serde_json::Value::Bool(print_runtime_deps));
    }
    if print_build_deps {
        data.insert("print_build_deps", serde_json::Value::Bool(print_build_deps));
    }

    hb.render("package", &data)
        .map_err(Error::from)
        .and_then(|r| writeln!(out, "{}", r).map_err(Error::from))
}

