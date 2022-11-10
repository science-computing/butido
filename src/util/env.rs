//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::Result;

use crate::util::EnvironmentVariableName;

pub fn parse_to_env(s: &str) -> Result<(EnvironmentVariableName, String)> {
    use crate::util::parser::*;
    let parser = {
        let key = (letters() + ((letters() | numbers() | under()).repeat(0..)))
            .collect()
            .convert(|b| String::from_utf8(b.to_vec()));

        let val = nonempty_string_with_optional_quotes()
            .collect()
            .convert(|b| String::from_utf8(b.to_vec()));

        (key + equal() + val).map(|((k, _), v)| (k, v))
    };

    match parser.parse(s.as_bytes()).map_err(|e| e.to_string()) {
        Err(s) => anyhow::bail!("Error during validation: '{}' is not a key-value pair", s),
        Ok((k, v)) => {
            Ok((EnvironmentVariableName::from(k), v))
        }
    }
}
