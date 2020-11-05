use std::path::Path;

use anyhow::Result;
use anyhow::Error;
use git2::Repository;

pub fn repo_is_clean(p: &Path) -> Result<bool> {
    Repository::open(p)
        .map_err(Error::from)
        .map(|r| r.state() == git2::RepositoryState::Clean)
}

pub fn get_repo_head_commit_hash(p: &Path) -> Result<String> {
    let r = Repository::open(p)?;
    let hash = r.head()?
        .peel(git2::ObjectType::Commit)?
        .id()
        .as_bytes()
        .to_vec();

    String::from_utf8(hash).map_err(Error::from)
}
