pub fn default_progress_format() -> String {
    String::from("[{elapsed_precise}] ({percent:>3}%): {bar:40.cyan/blue} | {msg}")
}

pub fn default_package_print_format() -> String {
    String::from(indoc::indoc!(r#"
            {{i}} - {{name}} : {{version}}
            Source: {{source_url}}
            Hash ({{source_hash_type}}): {{source_hash}}"
            {{#if print_system_deps}}System Deps: {{ system_deps }} {{/if}}
            {{#if print_system_runtime_deps}}System runtime Deps: {{ system_runtime_deps }} {{/if}}
            {{#if print_build_deps}}Build Deps: {{ build_deps }} {{/if}}
            {{#if print_runtime_deps}}Runtime Deps: {{ runtime_deps }} {{/if}}

    "#))
}

