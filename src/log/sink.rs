use anyhow::Result;

use crate::log::LogItem;

pub trait LogSink: Sized {
    fn log_item(&mut self, item: LogItem) -> Result<()>;

    fn close(self) -> Result<()> {
        Ok(())
    }
}

