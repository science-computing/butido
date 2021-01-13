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

#[derive(Debug, CopyGetters, Getters, Deserialize)]
pub struct ContainerConfig {
    #[getset(get_copy = "pub")]
    check_env_names: bool,

    #[getset(get = "pub")]
    allowed_env: Vec<EnvironmentVariableName>,
}

