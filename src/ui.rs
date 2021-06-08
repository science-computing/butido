//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Utility functions for the UI

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use handlebars::Handlebars;
use itertools::Itertools;
use log::error;

use crate::config::Configuration;
use crate::package::Package;
use crate::package::Script;
use crate::package::ScriptBuilder;
use crate::package::Shebang;

pub fn package_repo_cleanness_check(repo: &git2::Repository) -> Result<()> {
    if !crate::util::git::repo_is_clean(&repo)? {
        error!(
            "Repository not clean, refusing to go on: {}",
            repo.path().display()
        );
        Err(anyhow!(
            "Repository not clean, refusing to go on: {}",
            repo.path().display()
        ))
    } else {
        Ok(())
    }
}

pub struct PackagePrintFlags {
    pub print_all: bool,
    pub print_runtime_deps: bool,
    pub print_build_deps: bool,
    pub print_sources: bool,
    pub print_dependencies: bool,
    pub print_patches: bool,
    pub print_env: bool,
    pub print_flags: bool,
    pub print_allowed_images: bool,
    pub print_denied_images: bool,
    pub print_phases: bool,
    pub print_script: bool,
    pub script_line_numbers: bool,
    pub script_highlighting: bool,
}

impl PackagePrintFlags {
    // Helper to check whether any of the CLI args requested one of these flags.
    //
    // The print_build_deps and print_runtime_deps as well as the script_highlighting and
    // script_line_numbers is not included, because these only modify what to print and not whether
    // to print.
    fn print_any(&self) -> bool {
        self.print_all || {
            self.print_sources
                || self.print_dependencies
                || self.print_patches
                || self.print_env
                || self.print_flags
                || self.print_allowed_images
                || self.print_denied_images
                || self.print_phases
                || self.print_script
        }
    }
}

pub fn print_packages<'a, I>(
    out: &mut dyn Write,
    format: &str,
    iter: I,
    config: &Configuration,
    flags: &PackagePrintFlags,
) -> Result<()>
where
    I: Iterator<Item = &'a Package>,
{
    let mut hb = Handlebars::new();
    hb.register_escape_fn(handlebars::no_escape);
    hb.register_template_string("package", format)?;

    for (i, package) in iter.enumerate() {
        print_package(out, &hb, i, package, config, flags)?;
    }

    Ok(())
}

fn print_package(
    out: &mut dyn Write,
    hb: &Handlebars,
    i: usize,
    package: &Package,
    config: &Configuration,
    flags: &PackagePrintFlags,
) -> Result<()> {
    let script = ScriptBuilder::new(&Shebang::from(config.shebang().clone())).build(
        package,
        config.available_phases(),
        *config.strict_script_interpolation(),
    ).context("Rendering script for printing it failed")?;

    let script = crate::ui::script_to_printable(
        &script,
        flags.script_highlighting,
        config
            .script_highlight_theme()
            .as_ref()
            .ok_or_else(|| anyhow!("Highlighting for script enabled, but no theme configured"))?,
        flags.script_line_numbers,
    )?;

    let mut data = BTreeMap::new();
    data.insert("i", serde_json::Value::Number(serde_json::Number::from(i)));
    data.insert("p", serde_json::to_value(package)?);
    data.insert("script", serde_json::Value::String(script));
    data.insert("print_any", serde_json::Value::Bool(flags.print_any()));
    data.insert(
        "print_runtime_deps",
        serde_json::Value::Bool(flags.print_runtime_deps),
    );
    data.insert(
        "print_build_deps",
        serde_json::Value::Bool(flags.print_build_deps),
    );

    data.insert(
        "print_sources",
        serde_json::Value::Bool(flags.print_all || flags.print_sources),
    );
    data.insert(
        "print_dependencies",
        serde_json::Value::Bool(flags.print_all || flags.print_dependencies),
    );
    data.insert(
        "print_patches",
        serde_json::Value::Bool(flags.print_all || flags.print_patches),
    );
    data.insert(
        "print_env",
        serde_json::Value::Bool(flags.print_all || flags.print_env),
    );
    data.insert(
        "print_flags",
        serde_json::Value::Bool(flags.print_all || flags.print_flags),
    );
    data.insert(
        "print_allowed_images",
        serde_json::Value::Bool(flags.print_all || flags.print_allowed_images),
    );
    data.insert(
        "print_denied_images",
        serde_json::Value::Bool(flags.print_all || flags.print_denied_images),
    );
    data.insert(
        "print_phases",
        serde_json::Value::Bool(flags.print_all || flags.print_phases),
    );
    data.insert(
        "print_script",
        serde_json::Value::Bool(flags.print_all || flags.print_script),
    );

    hb.render("package", &data)
        .map_err(Error::from)
        .and_then(|r| writeln!(out, "{}", r).map_err(Error::from))
}

pub fn script_to_printable(
    script: &Script,
    highlight: bool,
    highlight_theme: &str,
    line_numbers: bool,
) -> Result<String> {
    let script = if highlight {
        let script = script.highlighted(highlight_theme);
        if line_numbers {
            script
                .lines_numbered()?
                .map(|(i, s)| format!("{:>4} | {}", i, s))
                .join("")
        } else {
            script.lines()?.join("")
        }
    } else if line_numbers {
        script
            .lines_numbered()
            .map(|(i, s)| format!("{:>4} | {}", i, s))
            .join("")
    } else {
        script.to_string()
    };

    Ok(script)
}

pub fn find_linter_command(repo_path: &Path, config: &Configuration) -> Result<Option<PathBuf>> {
    match config.script_linter().as_ref() {
        None => Ok(None),
        Some(linter) => {
            if linter.is_absolute() {
                Ok(Some(linter.to_path_buf()))
            } else {
                let linter = repo_path.join(linter);
                if !linter.is_file() {
                    Err(anyhow!(
                        "Cannot find linter command, searched in: {}",
                        linter.display()
                    ))
                } else {
                    Ok(Some(linter))
                }
            }
        }
    }
}
