//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Utility functions for the UI

use std::path::Path;
use std::path::PathBuf;

use anyhow::Result;
use anyhow::anyhow;
use itertools::Itertools;

use crate::config::Configuration;
use crate::package::Script;

mod package;
pub use crate::ui::package::*;

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
                .map(|(i, s)| format!("{i:>4} | {s}"))
                .join("")
        } else {
            script.lines()?.join("")
        }
    } else if line_numbers {
        script
            .lines_numbered()
            .map(|(i, s)| format!("{i:>4} | {s}"))
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
