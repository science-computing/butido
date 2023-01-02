//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::collections::HashMap;

use anyhow::Error;
use uuid::Uuid;

/// Get a `Display`able interface for a Map of errors
///
/// This is a helper trait for be able to display a `HashMap<Uuid, Error>`
/// in a `log::trace!()` call, for example
pub trait AsReceivedErrorDisplay {
    fn display_error_map(&self) -> ReceivedErrorDisplay<'_>;
}

impl AsReceivedErrorDisplay for HashMap<Uuid, Error> {
    fn display_error_map(&self) -> ReceivedErrorDisplay<'_> {
        ReceivedErrorDisplay(self)
    }
}


pub struct ReceivedErrorDisplay<'a>(&'a HashMap<Uuid, Error>);

impl<'a> std::fmt::Display for ReceivedErrorDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.iter().try_for_each(|(uuid, err)| writeln!(f, "{uuid}: {err}"))
    }
}

