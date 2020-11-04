use crate::filestore::Artifact;

/// TODO implement
#[derive(Debug)]
pub enum JobResource {
    Environment(String, String),
    Artifact(Artifact)
}

