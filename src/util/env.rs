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
use anyhow::Result;

use crate::util::EnvironmentVariableName;

pub fn parse_to_env(s: &str) -> Result<(EnvironmentVariableName, String)> {
    let v = s.split('=').collect::<Vec<_>>();
    Ok((
        EnvironmentVariableName::from(
            *v.first()
                .ok_or_else(|| anyhow!("Environment variable has no key: {s}"))?,
        ),
        String::from(
            *v.get(1)
                .ok_or_else(|| anyhow!("Environment variable has no key: {s}"))?,
        ),
    ))
}
