use crate::filestore::Artifact;

/// TODO implement
#[derive(Debug)]
pub enum JobResource {
    Environment(String, String),
    Artifact(Artifact)
}

impl JobResource {
    pub fn env(&self) -> Option<(&String, &String)> {
        match self {
            JobResource::Environment(k, v) => Some((k, v)),
            _ => None
        }
    }
    pub fn artifact(&self) -> Option<&Artifact> {
        match self {
            JobResource::Artifact(a) => Some(a),
            _ => None
        }
    }
}

