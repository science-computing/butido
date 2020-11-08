use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Error;
use anyhow::Result;

use crate::job::Job;
use crate::log::LogItem;
use crate::log::LogSink;

pub struct FileSink {
    file: File,
}

impl FileSink {
    fn new(path: &Path) -> Result<Self> {
        OpenOptions::new()
            .create(true)
            .append(true)
            .write(false)
            .open(path)
            .map(|file| FileSink { file })
            .map_err(Error::from)
    }
}

impl LogSink for FileSink {
    fn log_item(&mut self, item: &LogItem) -> Result<()> {
        let s = item.display()?;
        writeln!(self.file, "{}", s)?;
        Ok(())
    }
}

pub struct FileLogSinkFactory {
    root: PathBuf
}

impl FileLogSinkFactory {
    pub fn new(root: PathBuf) -> Self {
        FileLogSinkFactory { root }
    }

    pub fn new_file_sink(&self, job: &Job) -> Result<FileSink> {
        let now = chrono::offset::Local::now()
            .naive_local()
            .format("%Y-%m-%dT%H:%M:%S");

        trace!("Got current time: {}", now);
        let filename = format!("{}-{}", now, job.package().name());

        trace!("Building path from {} and {}", self.root.display(), filename);
        let p = self.root.join(filename);

        FileSink::new(&p)
    }
}

