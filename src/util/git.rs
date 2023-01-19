//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use git2::Repository;
use tracing::trace;

pub fn repo_is_clean(r: &Repository) -> Result<bool> {
    r.diff_index_to_workdir(None, None)
        .and_then(|d| d.stats())
        .map_err(Error::from)
        .map(|st| {
            trace!("Repo stats: {:?}", st);
            trace!("Repo state: {:?}", r.state());
            st.files_changed() == 0 && r.state() == git2::RepositoryState::Clean
        })
}

pub fn get_repo_head_commit_hash(r: &Repository) -> Result<String> {
    let s = r
        .head()
        .with_context(|| anyhow!("Getting HEAD from repository at {}", r.path().display()))?
        .peel_to_commit()
        .with_context(|| anyhow!("Failed to get commit hash: Not valid UTF8"))?
        .id()
        .to_string();

    trace!("Found git commit hash = {}", s);
    Ok(s)
}
