use std::path::Path;

use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use git2::Repository;

pub fn repo_is_clean(p: &Path) -> Result<bool> {
    Repository::open(p)
        .map_err(Error::from)
        .map(|r| r.state() == git2::RepositoryState::Clean)
}

pub fn get_repo_head_commit_hash(p: &Path) -> Result<String> {
    let r = Repository::open(p)
        .with_context(|| anyhow!("Opening repository at {}", p.display()))?;

    let s = r.head()
        .with_context(|| anyhow!("Getting HEAD from repository at {}", p.display()))?
        .shorthand()
        .ok_or_else(|| {
            anyhow!("Failed to get commit hash: Not valid UTF8")
        })?
        .to_owned();

    trace!("Found git commit hash = {}", s);
    Ok(s)
}
