//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::Error;
use anyhow::Result;
use colored::Colorize;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum LogItem {
    /// A line from the log, unmodified
    Line(Vec<u8>),

    /// A progress report
    Progress(usize),

    /// The name of the current phase the process is in
    CurrentPhase(String),

    /// The end-state of the process
    /// Either Ok or Error
    State(Result<(), String>),
}

impl LogItem {
    pub fn display(&self) -> Result<Display> {
        match self {
            LogItem::Line(s) => Ok(Display(String::from_utf8(s.to_vec())?.normal())),
            LogItem::Progress(u) => Ok(Display(format!("#BUTIDO:PROGRESS:{}", u).cyan())),
            LogItem::CurrentPhase(p) => Ok(Display(format!("#BUTIDO:PHASE:{}", p).cyan())),
            LogItem::State(Ok(())) => Ok(Display("#BUTIDO:STATE:OK".to_string().green())),
            LogItem::State(Err(s)) => Ok(Display(format!("#BUTIDO:STATE:ERR:{}", s).red())),
        }
    }

    pub fn raw(&self) -> Result<String> {
        match self {
            LogItem::Line(s) => String::from_utf8(s.to_vec()).map_err(Error::from),
            LogItem::Progress(u) => Ok(format!("#BUTIDO:PROGRESS:{}", u)),
            LogItem::CurrentPhase(p) => Ok(format!("#BUTIDO:PHASE:{}", p)),
            LogItem::State(Ok(())) => Ok("#BUTIDO:STATE:OK".to_string()),
            LogItem::State(Err(s)) => Ok(format!("#BUTIDO:STATE:ERR:{}", s)),
        }
    }
}

#[derive(parse_display::Display)]
#[display("{0}")]
pub struct Display(colored::ColoredString);
