use std::path::Path;

use anyhow::Result;
use anyhow::Error;
use git2::Repository;

pub fn get_repo_head_commit_hash(p: &Path) -> Result<String> {
    let r = Repository::open(p)?;
    let hash = r.head()?
        .peel(git2::ObjectType::Commit)?
        .id()
        .as_bytes()
        .to_vec();

    String::from_utf8(hash).map_err(Error::from)
}
