pub fn default_progress_format() -> String {
    String::from("[{elapsed_precise}] ({percent:>3}%): {bar:40.cyan/blue} | {msg}")
}

pub fn default_spinner_format() -> String {
    String::from("[{elapsed_precise}] {spinner} | {msg}")
}

pub fn default_package_print_format() -> String {
    String::from(indoc::indoc!(r#"
            {{i}} - {{p.name}} : {{p.version}}
            {{~ #if print_any }}
            ==================================

            {{~#if print_sources}}
            Sources:
                {{#each p.sources ~}}
                    {{~@key}} = {{this.url}} - {{this.hash.hash}} ({{this.hash.type}})
                {{/each~}}
            {{/if~}}
            {{~#if print_dependencies}}
            Dependencies:
                {{#if print_build_deps ~}}
                    {{~ #each p.dependencies.build}}
                        {{~ this}} (build)
                {{/each ~}}
                {{/if ~}}
                {{~ #if print_runtime_deps ~}}
                    {{~ #each p.dependencies.runtime}}
                        {{~ this}} (runtime)
                {{/each ~}}
                {{/if ~}}
            {{/if}}
            {{~#if print_patches}}
            Patches:
                {{#each p.patches}}{{this}},
                {{/each~}}
            {{/if}}
            {{~#if print_env}}
            Environment:
                {{#each p.environment}}{{@key}}={{this}}
                {{/each~}}
            {{/if~}}
            {{~#if print_flags}}
            Flags:
                {{#each p.flags}}{{this}}
                {{/each~}}
            {{/if~}}
            {{~#if print_allowed_images}}
            Only supported on:
                {{#each p.allowed_images}}{{this}}
                {{/each~}}
            {{/if~}}
            {{~#if print_denied_images}}
            Denied on:
                {{#each p.denied_images}}{{this}}
                {{/each~}}
            {{/if~}}
            {{~#if print_phases}}
            Phases:
                {{#each p.phases}}{{@key}}
                {{/each~}}
            {{/if~}}
            {{~#if print_script}}
            {{script}}
            {{/if~}}
            {{~ /if ~}}
    "#))
}

pub fn default_strict_script_interpolation() -> bool {
    true
}

pub fn default_script_shebang() -> String {
    String::from("#!/bin/bash")
}

pub fn default_build_error_lines() -> usize {
    10
}

