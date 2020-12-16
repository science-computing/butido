use anyhow::Result;

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
            LogItem::Line(s)         => Ok(Display(String::from_utf8(s.to_vec())?)),
            LogItem::Progress(u)     => Ok(Display(format!("#BUTIDO:PROGRESS:{}", u))),
            LogItem::CurrentPhase(p) => Ok(Display(format!("#BUTIDO:PHASE:{}", p))),
            LogItem::State(Ok(()))   => Ok(Display(format!("#BUTIDO:STATE:OK"))),
            LogItem::State(Err(s))   => Ok(Display(format!("#BUTIDO:STATE:ERR:{}", s))),
        }
    }
}

#[derive(parse_display::Display)]
#[display("{0}")]
pub struct Display(String);

