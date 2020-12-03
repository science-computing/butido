pub fn default_progress_format() -> String {
    String::from("[{elapsed_precise}] ({percent:>3}%): {bar:40.cyan/blue} | {msg}")
}

pub fn default_package_print_format() -> String {
    String::from(indoc::indoc!(r#"
            {{i}} - {{p.name}} : {{p.version}}
            {{#each p.sources}}Source: {{this.url}} - {{this.hash.hash}} ({{this.hash.type}}){{/each}}
            {{#if print_system_deps}}System Deps: {{ p.dependencies.system }} {{/if}}
            {{#if print_system_runtime_deps}}System runtime Deps: {{ p.dependencies.system_runtime }} {{/if}}
            {{#if print_build_deps}}Build Deps: {{ p.dependencies.build }} {{/if}}
            {{#if print_runtime_deps}}Runtime Deps: {{ p.dependencies.runtime }} {{/if}}

    "#))
}

