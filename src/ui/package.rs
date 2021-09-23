//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::borrow::Borrow;
use std::collections::BTreeMap;

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use handlebars::Handlebars;

use crate::config::Configuration;
use crate::package::Package;
use crate::package::ScriptBuilder;
use crate::package::Shebang;

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


pub trait PreparePrintable<'a>
    where Self: Borrow<Package> + Sized
{
    fn prepare_print(self, config: &'a Configuration, flags: &'a PackagePrintFlags, handlebars: &'a Handlebars<'a>, i: usize) -> PreparePrintPackage<'a, Self>;
}

impl<'a, P> PreparePrintable<'a> for P
    where P: Borrow<Package>
{
    fn prepare_print(self, config: &'a Configuration, flags: &'a PackagePrintFlags, handlebars: &'a Handlebars<'a>, i: usize) -> PreparePrintPackage<'a, P> {
        PreparePrintPackage {
            package: self,
            config,
            flags,
            handlebars,
            i,
        }
    }
}

pub struct PreparePrintPackage<'a, P: Borrow<Package>> {
    package: P,
    config: &'a Configuration,
    flags: &'a PackagePrintFlags,
    handlebars: &'a Handlebars<'a>,
    i: usize,
}


pub fn handlebars_for_package_printing(format: &str) -> Result<Handlebars> {
    let mut hb = Handlebars::new();
    hb.register_escape_fn(handlebars::no_escape);
    hb.register_template_string("package", format)?;
    Ok(hb)
}

impl<'a, P: Borrow<Package>> PreparePrintPackage<'a, P> {
    pub fn into_displayable(self) -> Result<PrintablePackage> {
        let script = ScriptBuilder::new(&Shebang::from(self.config.shebang().clone())).build(
            self.package.borrow(),
            self.config.available_phases(),
            *self.config.strict_script_interpolation(),
        ).context("Rendering script for printing it failed")?;

        let script = crate::ui::script_to_printable(
            &script,
            self.flags.script_highlighting,
            self.config
                .script_highlight_theme()
                .as_ref()
                .ok_or_else(|| anyhow!("Highlighting for script enabled, but no theme configured"))?,
            self.flags.script_line_numbers,
        )?;

        let mut data = BTreeMap::new();
        data.insert("i", serde_json::Value::Number(serde_json::Number::from(self.i)));
        data.insert("p", serde_json::to_value(self.package.borrow())?);
        data.insert("script", serde_json::Value::String(script));
        data.insert("print_any", serde_json::Value::Bool(self.flags.print_any()));
        data.insert(
            "print_runtime_deps",
            serde_json::Value::Bool(self.flags.print_runtime_deps),
        );
        data.insert(
            "print_build_deps",
            serde_json::Value::Bool(self.flags.print_build_deps),
        );

        data.insert(
            "print_sources",
            serde_json::Value::Bool(self.flags.print_all || self.flags.print_sources),
        );
        data.insert(
            "print_dependencies",
            serde_json::Value::Bool(self.flags.print_all || self.flags.print_dependencies),
        );
        data.insert(
            "print_patches",
            serde_json::Value::Bool(self.flags.print_all || self.flags.print_patches),
        );
        data.insert(
            "print_env",
            serde_json::Value::Bool(self.flags.print_all || self.flags.print_env),
        );
        data.insert(
            "print_flags",
            serde_json::Value::Bool(self.flags.print_all || self.flags.print_flags),
        );
        data.insert(
            "print_allowed_images",
            serde_json::Value::Bool(self.flags.print_all || self.flags.print_allowed_images),
        );
        data.insert(
            "print_denied_images",
            serde_json::Value::Bool(self.flags.print_all || self.flags.print_denied_images),
        );
        data.insert(
            "print_phases",
            serde_json::Value::Bool(self.flags.print_all || self.flags.print_phases),
        );
        data.insert(
            "print_script",
            serde_json::Value::Bool(self.flags.print_all || self.flags.print_script),
        );

        let string = self.handlebars.render("package", &data)?;
        Ok(PrintablePackage { string })
    }
}

pub struct PrintablePackage { string: String }

impl std::fmt::Display for PrintablePackage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.string)
    }
}

