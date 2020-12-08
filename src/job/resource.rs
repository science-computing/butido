use crate::filestore::Artifact;
use crate::util::EnvironmentVariableName;

#[derive(Clone, Debug)]
pub enum JobResource {
    Environment(EnvironmentVariableName, String),
    Artifact(Artifact)
}

impl From<(EnvironmentVariableName, String)> for JobResource {
    fn from(tpl: (EnvironmentVariableName, String)) -> Self {
        JobResource::Environment(tpl.0, tpl.1)
    }
}

impl JobResource {
    pub fn env(&self) -> Option<(&EnvironmentVariableName, &String)> {
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

