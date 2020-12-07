pub fn default_progress_format() -> String {
    String::from("[{elapsed_precise}] ({percent:>3}%): {bar:40.cyan/blue} | {msg}")
}

pub fn default_package_print_format() -> String {
    String::from(indoc::indoc!(r#"
            {{i}} - {{p.name}} : {{p.version}}
            {{#each p.sources}}Source: {{this.url}} - {{this.hash.hash}} ({{this.hash.type}}){{/each}}
            {{#if print_build_deps}}Build Deps: {{ p.dependencies.build }} {{/if}}
            {{#if print_runtime_deps}}Runtime Deps: {{ p.dependencies.runtime }} {{/if}}

    "#))
}

pub fn default_strict_script_interpolation() -> bool {
    true
}

pub fn default_script_shebang() -> String {
    String::from("#!/bin/bash")
}

