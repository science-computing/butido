use std::path::PathBuf;

/// TODO implement
#[derive(Debug)]
pub enum JobResource {
    Environment(String, String),
    Path(PathBuf)
}

