//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use getset::CopyGetters;
use getset::Getters;
use serde::Deserialize;

use crate::util::EnvironmentVariableName;

/// The configuration for the containers
#[derive(Debug, CopyGetters, Getters, Deserialize)]
pub struct ContainerConfig {
    /// check environment names whether they're allowed
    #[getset(get_copy = "pub")]
    check_env_names: bool,

    /// Allowed environment variables (names)
    #[getset(get = "pub")]
    allowed_env: Vec<EnvironmentVariableName>,

    /// Pass the current git author to the container
    /// This can be used to the the "packager" name in a package, for example
    #[getset(get = "pub")]
    git_author: Option<EnvironmentVariableName>,

    /// Pass the current git hash to the container
    #[getset(get = "pub")]
    git_commit_hash: Option<EnvironmentVariableName>,
}
