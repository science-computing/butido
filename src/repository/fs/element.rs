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

use crate::repository::fs::path::PathComponent;

/// One element in the tree inside FileSystemRepresentation
///
/// This is either a File, or a Directory that contains more (Files or Directories).
#[derive(Debug)]
pub enum Element {
    File(String),
    Dir(HashMap<PathComponent, Element>),
}

impl Element {
    /// Helper fn to get the directory contents of the element, if the element is an Element::Dir
    pub fn get_map_mut(&mut self) -> Option<&mut HashMap<PathComponent, Element>> {
        match self {
            Element::File(_) => None,
            Element::Dir(hm) => Some(hm),
        }
    }
}
