//
// Copyright (c) 2020-2022 science+computing ag and other contributors
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
#[serde(deny_unknown_fields)]
pub struct ContainerConfig {
    /// Whether to check if environment variables are allowed (i.e., if their
    /// names are listed in `allowed_env`).
    #[getset(get_copy = "pub")]
    check_env_names: bool,

    /// Allowed environment variables (names)
    #[getset(get = "pub")]
    allowed_env: Vec<EnvironmentVariableName>,

    /// Pass the current Git author to the container
    /// This can be used for the "packager" name in a package, for example
    #[getset(get = "pub")]
    git_author: Option<EnvironmentVariableName>,

    /// Pass the current Git hash to the container
    #[getset(get = "pub")]
    git_commit_hash: Option<EnvironmentVariableName>,
}
