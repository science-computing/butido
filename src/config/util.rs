//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

//! This module contains default functions that are called by serde when deserializing the
//! configuration and having to use default values.

/// The default progress bar format
pub fn default_progress_format() -> String {
    String::from("{elapsed_precise} {percent:>3}% {bar:5.cyan/blue} | {msg}")
}

/// The default format that is used to print one package
pub fn default_package_print_format() -> String {
    String::from(indoc::indoc!(
        r#"
            {{i}} - {{p.name}} : {{p.version}}
            {{~ #if print_any}}

            ==================================

            {{#if print_sources}}
            Sources:
            {{#each p.sources}}
                {{@key}} = {{this.url}} - {{this.hash.hash}} ({{this.hash.type}})
            {{/each}}
            {{/if~}}

            {{#if print_dependencies}}
            Dependencies:
            {{#if print_build_deps ~}}
            {{#each p.dependencies.build}}
                {{this}} (build)
            {{/each}}
            {{/if}}
            {{#if print_runtime_deps ~}}
            {{#each p.dependencies.runtime}}
                {{this}} (runtime)
            {{/each}}
            {{/if}}
            {{/if~}}

            {{#if print_patches}}
            Patches:
            {{#each p.patches}}
                {{this}},
            {{/each~}}
            {{/if~}}

            {{#if print_env}}
            Environment:
            {{#each p.environment}}
                {{@key}}={{this}}
            {{/each~}}
            {{/if~}}

            {{~#if print_flags}}
            Flags:
            {{#each p.flags}}
                {{this}}
            {{/each}}
            {{/if~}}

            {{~#if print_allowed_images}}
            Only supported on:
            {{#each p.allowed_images}}
                {{this}}
            {{/each}}
            {{/if~}}

            {{~#if print_denied_images}}
            Denied on:
            {{#each p.denied_images}}
                {{this}}
            {{/each}}
            {{/if~}}

            {{#if print_phases}}
            Phases:
            {{#each p.phases}}
                {{@key}}
            {{/each}}
            {{/if~}}

            {{~#if print_script}}
            {{script}}
            {{/if~}}
            {{~ /if ~}}
        "#
    ))
}

/// The default value for whether strict script interpolation should be used
pub fn default_strict_script_interpolation() -> bool {
    true
}

/// The default value for the shebang
pub fn default_script_shebang() -> String {
    String::from("#!/bin/bash")
}

/// The default value for the number of log lines that should be printed if a build fails
pub fn default_build_error_lines() -> usize {
    10
}

/// The default value for the number of results/rows that should be returned for DB queries that
/// list things (LIMIT)
pub fn default_database_query_limit() -> usize {
    10
}
