//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! Module that contains all types and functionality that has to do with a package.

mod dependency;
pub use dependency::*;

mod name;
pub use name::*;

#[allow(clippy::module_inception)]
mod package;
pub use package::*;

mod phase;
pub use phase::*;

mod script;
pub use script::*;

mod source;
pub use source::*;

mod tree;
pub use tree::*;

mod version;
pub use version::*;
