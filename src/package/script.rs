//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::process::ExitStatus;

use anyhow::anyhow;
use anyhow::Context as AnyhowContext;
use anyhow::Error;
use anyhow::Result;
use handlebars::{
    Context, Handlebars, Helper, HelperDef, HelperResult, JsonRender, Output, PathAndJson,
    RenderContext, RenderError,
};
use log::trace;
use serde::Deserialize;
use serde::Serialize;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use tokio::process::Command;

use crate::package::Package;
use crate::package::Phase;
use crate::package::PhaseName;

#[derive(parse_display::Display, Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
#[display("{0}")]
pub struct Script(String);

impl From<String> for Script {
    fn from(s: String) -> Script {
        Script(s)
    }
}

#[derive(Clone, Debug)]
pub struct Shebang(String);

impl Script {
    pub fn highlighted<'a>(&'a self, script_theme: &'a str) -> HighlightedScript<'a> {
        HighlightedScript::new(self, script_theme)
    }

    pub fn lines_numbered(&self) -> impl Iterator<Item = (usize, &str)> {
        self.0.lines().enumerate().map(|(n, l)| (n + 1, l))
    }

    pub async fn lint(&self, mut cmd: Command) -> Result<(ExitStatus, String, String)> {
        use tokio::io::AsyncWriteExt;
        use tokio::io::BufWriter;

        let mut child = cmd
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("Spawning subprocess for linting package script")?;

        trace!("Child = {:?}", child);

        {
            let stdin = child.stdin.take().ok_or_else(|| anyhow!("No stdin"))?;
            let mut writer = BufWriter::new(stdin);
            writer
                .write_all(self.0.as_bytes())
                .await
                .context("Writing package script to STDIN of subprocess")?;

            writer
                .flush()
                .await
                .context("Flushing STDIN of subprocess")?;
            trace!("Script written");
        }

        trace!("Waiting for child...");
        let out = child
            .wait_with_output()
            .await
            .context("Waiting for subprocess")?;

        Ok((
            out.status,
            String::from_utf8(out.stdout)?,
            String::from_utf8(out.stderr)?,
        ))
    }
}

#[derive(Debug)]
pub struct HighlightedScript<'a> {
    script: &'a Script,
    script_theme: &'a str,

    ps: SyntaxSet,
    ts: ThemeSet,
}

impl<'a> HighlightedScript<'a> {
    fn new(script: &'a Script, script_theme: &'a str) -> Self {
        HighlightedScript {
            script,
            script_theme,

            ps: SyntaxSet::load_defaults_newlines(),
            ts: ThemeSet::load_defaults(),
        }
    }

    pub fn lines(&'a self) -> Result<impl Iterator<Item = String> + 'a> {
        let syntax = self
            .ps
            .find_syntax_by_first_line(&self.script.0)
            .ok_or_else(|| anyhow!("Failed to load syntax for highlighting script"))?;

        let theme = self
            .ts
            .themes
            .get(self.script_theme)
            .ok_or_else(|| anyhow!("Theme not available: {}", self.script_theme))?;

        let mut h = HighlightLines::new(syntax, theme);

        Ok({
            LinesWithEndings::from(&self.script.0).map(move |line| {
                let ranges: Vec<(Style, &str)> = h.highlight(line, &self.ps);
                as_24_bit_terminal_escaped(&ranges[..], true)
            })
        })
    }

    pub fn lines_numbered(&'a self) -> Result<impl Iterator<Item = (usize, String)> + 'a> {
        self.lines().map(|iter| iter.enumerate().map(|(n, l)| (n + 1, l)))
    }
}

impl From<String> for Shebang {
    fn from(s: String) -> Self {
        Shebang(s)
    }
}

impl AsRef<str> for Script {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

pub struct ScriptBuilder<'a> {
    shebang: &'a Shebang,
}

impl<'a> ScriptBuilder<'a> {
    pub fn new(shebang: &'a Shebang) -> Self {
        ScriptBuilder { shebang }
    }

    pub fn build(
        self,
        package: &Package,
        phaseorder: &[PhaseName],
        strict_mode: bool,
    ) -> Result<Script> {
        let mut script = format!("{shebang}\n", shebang = self.shebang.0);

        for name in phaseorder {
            match package.phases().get(name) {
                Some(Phase::Text(text)) => {
                    use unindent::Unindent;

                    script.push_str(&indoc::formatdoc!(
                        r#"
                        ### phase {}
                        {}
                        ### / {} phase
                    "#,
                        name.as_str(),
                        // whack hack: insert empty line on top because unindent ignores the
                        // indentation of the first line, see commit message for more info
                        format!("\n{}", text).unindent(),
                        name.as_str(),
                    ));

                    script.push('\n');
                }

                // TODO: Support path embedding
                // (requires possibility to have stuff in Script type that gets copied to
                // container)
                Some(Phase::Path(pb)) => {
                    script.push_str(&format!(
                        r#"
                        # Phase (from file {path}): {name}
                        # NOT SUPPORTED YET
                        exit 1
                    "#,
                        path = pb.display(),
                        name = name.as_str()
                    ));
                    script.push('\n');
                }

                None => {
                    script.push_str(&format!(
                        "# No script for phase: {name}",
                        name = name.as_str()
                    ));
                    script.push('\n');
                }
            }
        }

        Self::interpolate_package(script, package, strict_mode).map(Script)
    }

    fn interpolate_package(script: String, package: &Package, strict_mode: bool) -> Result<String> {
        let mut hb = Handlebars::new();
        hb.register_escape_fn(handlebars::no_escape);
        hb.register_template_string("script", script)?;
        hb.register_helper("phase", Box::new(PhaseHelper));
        hb.register_helper("state", Box::new(StateHelper));
        hb.register_helper("progress", Box::new(ProgressHelper));
        hb.register_helper("join", Box::new(JoinHelper));
        hb.register_helper("joinwith", Box::new(JoinWithHelper));
        hb.set_strict_mode(strict_mode);

        #[cfg(debug_assertions)]
        {
            trace!("Rendering Package: {:?}", package.debug_details());
        }

        hb.render("script", package)
            .with_context(|| anyhow!("Rendering script for package {} {} failed", package.name(), package.version()))
            .map_err(Error::from)
    }
}

#[derive(Clone, Copy)]
struct PhaseHelper;

impl HelperDef for PhaseHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _rc: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        h.param(0)
            .ok_or_else(|| RenderError::new("Required parameter missing: phase name"))?
            .value()
            .as_str()
            .ok_or_else(|| RenderError::new("Required parameter must be a string: phase name"))
            .and_then(|phase_name| {
                out.write("echo '#BUTIDO:PHASE:")?;
                out.write(phase_name)?;
                out.write("'")?;
                Ok(())
            })
    }
}

#[derive(Clone, Copy)]
struct StateHelper;

impl HelperDef for StateHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _rc: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        h.param(0)
            .ok_or_else(|| RenderError::new("Required parameter missing: state"))?
            .value()
            .as_str()
            .ok_or_else(|| RenderError::new("Required parameter must be a string: state"))
            .and_then(|state| match state {
                "OK" => {
                    out.write("echo '#BUTIDO:STATE:OK'")?;
                    Ok(())
                }
                "ERR" => {
                    let state_msg = h.param(1).ok_or_else(|| {
                        RenderError::new("Required parameter missing: state message")
                    })?;
                    out.write("echo '#BUTIDO:STATE:ERR:")?;
                    out.write(state_msg.value().render().as_ref())?;
                    out.write("'")?;
                    Ok(())
                }
                other => Err(RenderError::new(format!(
                    "Parameter must bei either 'OK' or 'ERR', '{}' is invalid",
                    other
                ))),
            })
    }
}

#[derive(Clone, Copy)]
struct ProgressHelper;

impl HelperDef for ProgressHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _rc: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        h.param(0)
            .ok_or_else(|| RenderError::new("Required parameter missing: progress"))?
            .value()
            .as_i64()
            .ok_or_else(|| RenderError::new("Required parameter must be a number: progress"))
            .and_then(|progress| {
                out.write("echo '#BUTIDO:PROGRESS:")?;
                out.write(&progress.to_string())?;
                out.write("'")?;
                Ok(())
            })
    }
}

#[derive(Clone, Copy)]
struct JoinHelper;

impl HelperDef for JoinHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _rc: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        joinstrs("", h.params().iter(), out)
    }
}

#[derive(Clone, Copy)]
struct JoinWithHelper;

impl HelperDef for JoinWithHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper,
        _: &Handlebars,
        _: &Context,
        _rc: &mut RenderContext,
        out: &mut dyn Output,
    ) -> HelperResult {
        let joiner = h
            .param(0)
            .ok_or_else(|| RenderError::new("Required parameter missing: Join string"))?
            .value()
            .as_str()
            .ok_or_else(|| RenderError::new("Required parameter must be a string: joinstr"))?;

        joinstrs(joiner, h.params().iter().skip(1), out)
    }
}

fn joinstrs<'reg: 'rc, 'rc, I>(with: &str, params: I, out: &mut dyn Output) -> HelperResult
where
    I: Iterator<Item = &'rc PathAndJson<'reg, 'rc>>,
{
    use itertools::Itertools;
    use std::result::Result as RResult;

    let s = params
        .map(|p| {
            p.value()
                .as_str()
                .ok_or_else(|| RenderError::new("All parameters must be string"))
        })
        .collect::<RResult<Vec<&str>, RenderError>>()?
        .into_iter()
        .join(with);

    out.write(&s)?;
    Ok(())
}
