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
    State(Result<String, String>),
}

impl LogItem {
    pub fn display(&self) -> Result<Display> {
        match self {
            LogItem::Line(s)         => Ok(Display(String::from_utf8(s.to_vec())?)),
            LogItem::Progress(u)     => Ok(Display(format!("#BUTIDO:PROGRESS:{}", u))),
            LogItem::CurrentPhase(p) => Ok(Display(format!("#BUTIDO:PHASE:{}", p))),
            LogItem::State(Ok(s))    => Ok(Display(format!("#BUTIDO:STATE:OK:{}", s))),
            LogItem::State(Err(s))   => Ok(Display(format!("#BUTIDO:STATE:ERR:{}", s))),
        }
    }
}

pub struct Display(String);

impl Display {
    pub fn to_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.0)
    }
}

