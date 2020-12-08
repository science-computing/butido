//! Utility functions for the UI

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use handlebars::Handlebars;
use itertools::Itertools;
use log::error;
use syntect::easy::HighlightLines;
use syntect::highlighting::{ThemeSet, Style};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

use crate::package::Package;
use crate::package::ScriptBuilder;
use crate::package::Shebang;
use crate::config::Configuration;

pub fn package_repo_cleanness_check(repo_path: &Path) -> Result<()> {
    if !crate::util::git::repo_is_clean(&repo_path)? {
        error!("Repository not clean, refusing to go on: {}", repo_path.display());
        Err(anyhow!("Repository not clean, refusing to go on: {}", repo_path.display()))
    } else {
        Ok(())
    }
}

pub struct PackagePrintFlags {
    pub print_runtime_deps: bool,
    pub print_build_deps: bool,
    pub print_sources: bool,
    pub print_dependencies: bool,
    pub print_patches: bool,
    pub print_env: bool,
    pub print_flags: bool,
    pub print_deny_images: bool,
    pub print_phases: bool,
    pub print_script: bool,
    pub script_line_numbers: bool,
    pub script_highlighting: bool,
}

pub fn print_packages<'a, I>(out: &mut dyn Write,
                             format: &str,
                             iter: I,
                             config: &Configuration,
                             flags: &PackagePrintFlags)
-> Result<()>
    where I: Iterator<Item = &'a Package>
{
    let mut hb = Handlebars::new();
    hb.register_template_string("package", format)?;

    for (i, package) in iter.enumerate() {
        print_package(out, &hb, i, package, config, flags)?;
    }

    Ok(())
}

fn print_package(out: &mut dyn Write,
                 hb: &Handlebars,
                 i: usize,
                 package: &Package,
                 config: &Configuration,
                 flags: &PackagePrintFlags)
    -> Result<()>
{
    let script = ScriptBuilder::new(&Shebang::from(config.shebang().clone()))
        .build(package, config.available_phases(), *config.strict_script_interpolation())?;

    let script = crate::ui::script_to_printable(script.as_ref(),
        flags.script_highlighting,
        config.script_highlight_theme().as_ref(),
        flags.script_line_numbers)?;

    let mut data = BTreeMap::new();
    data.insert("i"                  , serde_json::Value::Number(serde_json::Number::from(i)));
    data.insert("p"                  , serde_json::to_value(package)?);
    data.insert("script"             , serde_json::Value::String(script));
    data.insert("print_runtime_deps" , serde_json::Value::Bool(flags.print_runtime_deps));
    data.insert("print_build_deps"   , serde_json::Value::Bool(flags.print_build_deps));
    data.insert("print_sources"      , serde_json::Value::Bool(flags.print_sources));
    data.insert("print_dependencies" , serde_json::Value::Bool(flags.print_dependencies));
    data.insert("print_patches"      , serde_json::Value::Bool(flags.print_patches));
    data.insert("print_env"          , serde_json::Value::Bool(flags.print_env));
    data.insert("print_flags"        , serde_json::Value::Bool(flags.print_flags));
    data.insert("print_deny_images"  , serde_json::Value::Bool(flags.print_deny_images));
    data.insert("print_phases"       , serde_json::Value::Bool(flags.print_phases));
    data.insert("print_script"       , serde_json::Value::Bool(flags.print_script));


    hb.render("package", &data)
        .map_err(Error::from)
        .and_then(|r| writeln!(out, "{}", r).map_err(Error::from))
}

pub fn script_to_printable(script: &str,
                          highlight: bool,
                          highlight_theme: Option<&String>,
                          line_numbers: bool)
    -> Result<String>
{
    if highlight {
        if let Some(configured_theme) = highlight_theme {
            // Load these once at the start of your program
            let ps = SyntaxSet::load_defaults_newlines();
            let ts = ThemeSet::load_defaults();

            let syntax = ps
                .find_syntax_by_first_line(script)
                .ok_or_else(|| anyhow!("Failed to load syntax for highlighting script"))?;

            let theme = ts
                .themes
                .get(configured_theme)
                .ok_or_else(|| anyhow!("Theme not available: {}", configured_theme))?;

            let mut h = HighlightLines::new(syntax, &theme);

            let output = LinesWithEndings::from(script)
                .enumerate()
                .map(|(i, line)| {
                    let ranges: Vec<(Style, &str)> = h.highlight(line, &ps);
                    if line_numbers {
                        format!("{:>4} | {}", i, as_24_bit_terminal_escaped(&ranges[..], true))
                    } else {
                        as_24_bit_terminal_escaped(&ranges[..], true)
                    }
                })
                .join("");

            Ok(output)
        } else {
            Err(anyhow!("Highlighting for script enabled, but no theme configured"))
        }
    } else {
        if line_numbers {
            Ok({
                script.lines()
                    .enumerate()
                    .map(|(i, s)| format!("{:>4} | {}", i, s))
                    .join("\n")
            })
        } else {
            Ok(script.to_string())
        }
    }
}

