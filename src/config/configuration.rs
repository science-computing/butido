//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::ops::Deref;

use crate::config::NotValidatedConfiguration;

/// A valid configuration (validated via NotValidatedConfiguration::validate())
#[derive(Debug)]
pub struct Configuration {
    pub(in crate::config) inner: NotValidatedConfiguration,
}

impl Deref for Configuration {
    type Target = NotValidatedConfiguration;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
