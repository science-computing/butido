use std::convert::TryInto;

use anyhow::Result;
use anyhow::Error;

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
    State(Result<String, String>),
}

impl TryInto<String> for LogItem {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<String> {
        match self {
            LogItem::Line(v)         => String::from_utf8(v).map_err(Error::from),
            LogItem::Progress(u)     => Ok(format!("#BUTIDO:PROGRESS:{}", u)),
            LogItem::CurrentPhase(p) => Ok(format!("#BUTIDO:PHASE:{}", p)),
            LogItem::State(Ok(s))    => Ok(format!("#BUTIDO:STATE:OK:{}", s)),
            LogItem::State(Err(s))   => Ok(format!("#BUTIDO:STATE:ERR:{}", s)),
        }
    }
}

