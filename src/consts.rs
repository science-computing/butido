//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

/// The path to the directory inside the container where the inputs for the script run are copied
/// to.
pub const INPUTS_DIR_PATH: &str = "/inputs";

/// The path to the directory inside the container where the outputs of a compile job must be
/// located after the script was run
pub const OUTPUTS_DIR_PATH: &str = "/outputs";
pub const OUTPUTS_DIR_NAME: &str = "outputs";

pub const PATCH_DIR_PATH: &str = "/patches";

/// The path where the script that is executed inside the container is copied to.
pub const SCRIPT_PATH: &str = "/script";
