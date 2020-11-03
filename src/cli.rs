use clap_v3 as clap;
use clap::App;
use clap::Arg;
use clap::crate_authors;
use clap::crate_version;

// Helper types to ship around stringly typed clap API.
pub const IDENT_DEPENDENCY_TYPE_SYSTEM: &'static str         = "system";
pub const IDENT_DEPENDENCY_TYPE_SYSTEM_RUNTIME: &'static str = "system-runtime";
pub const IDENT_DEPENDENCY_TYPE_BUILD: &'static str          = "build";
pub const IDENT_DEPENDENCY_TYPE_RUNTIME: &'static str        = "runtime";

pub fn cli<'a>() -> App<'a> {
    App::new("butido")
        .author(crate_authors!())
        .version(crate_version!())
        .about("Generic Build Orchestration System for building linux packages with docker")

        .arg(Arg::with_name("database_host")
            .required(false)
            .multiple(false)
            .long("db-url")
            .value_name("HOST")
            .help("Overwrite the database host set via configuration. Can also be overriden via environment, but this setting has presendence.")
        )
        .arg(Arg::with_name("database_port")
            .required(false)
            .multiple(false)
            .long("db-port")
            .value_name("PORT")
            .help("Overwrite the database port set via configuration. Can also be overriden via environment, but this setting has presendence.")
        )
        .arg(Arg::with_name("database_user")
            .required(false)
            .multiple(false)
            .long("db-user")
            .value_name("USER")
            .help("Overwrite the database user set via configuration. Can also be overriden via environment, but this setting has presendence.")
        )
        .arg(Arg::with_name("database_password")
            .required(false)
            .multiple(false)
            .long("db-password")
            .alias("db-pw")
            .value_name("PASSWORD")
            .help("Overwrite the database password set via configuration. Can also be overriden via environment, but this setting has presendence.")
        )
        .arg(Arg::with_name("database_name")
            .required(false)
            .multiple(false)
            .long("db-name")
            .value_name("NAME")
            .help("Overwrite the database name set via configuration. Can also be overriden via environment, but this setting has presendence.")
        )


        .subcommand(App::new("build")
            .about("Build packages in containers")

            .arg(Arg::with_name("package_name")
                .required(true)
                .multiple(false)
                .index(1)
            )
            .arg(Arg::with_name("package_version")
                .required(false)
                .multiple(false)
                .index(2)
            )

            .arg(Arg::with_name("env")
                .required(false)
                .multiple(true)
                .short('E')
                .long("env")
                .validator(env_pass_validator)
                .help("Pass these variables to each build job (expects \"key=value\" or name of variable available in ENV)")
            )

            .arg(Arg::with_name("image")
                .required(true)
                .multiple(false)
                .short('I')
                .long("image")
                .help("Name of the docker image to use")
            )

            .arg(Arg::with_name("overwrite_release_dir")
                .required(true)
                .multiple(false)
                .long("realease-dir")
                .help("Overwrite the release directory. This is not recommended. Use the config file instead.")
            )

            .arg(Arg::with_name("overwrite_staging_dir")
                .required(true)
                .multiple(false)
                .short('S')
                .long("staging-dir")
                .help("Overwrite the staging directory.")
            )
        )

        .subcommand(App::new("what-depends")
            .about("List all packages that depend on a specific package")
            .arg(Arg::with_name("package_name")
                .required(true)
                .multiple(false)
                .index(1)
                .help("The name of the package")
            )
            .arg(Arg::with_name("dependency_type")
                .required(false)
                .multiple(true)
                .takes_value(true)
                .short('t')
                .long("type")
                .value_name("DEPENDENCY_TYPE")
                .possible_values(&[
                    IDENT_DEPENDENCY_TYPE_SYSTEM,
                    IDENT_DEPENDENCY_TYPE_SYSTEM_RUNTIME,
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .default_values(&[
                    IDENT_DEPENDENCY_TYPE_SYSTEM,
                    IDENT_DEPENDENCY_TYPE_SYSTEM_RUNTIME,
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .help("Specify which dependency types are to be checked. By default, all are checked")
            )
            .arg(Arg::with_name("list-format")
                .required(false)
                .multiple(false)
                .short('f')
                .long("format")
                .default_value("{{i}} - {{name}} - {{version}} - {{source_url}} - {{source_hash_type}}:{{source_hash}}")
                .help("The format to print the found packages with.")
                .long_help(&indoc::indoc!(r#"
                    The format to print the found packages with.

                    Possible tokens are:
                        i                   - Line number
                        name                - Name of the package
                        version             - Version of the package
                        source_url          - URL from where the source was retrieved
                        source_hash_type    - Type of hash noted in the package
                        source_hash         - Hash of the sources
                        system_deps         - System dependencies, as list
                        system_runtime_deps - System runtime dependencies, as list
                        build_deps          - Build dependencies, as list
                        runtime_deps        - Runtime dependencies, as list

                    Handlebars modifiers are available.
                "#))
            )
        )
        .subcommand(App::new("dependencies-of")
            .alias("depsof")
            .about("List the depenendcies of a package")
            .arg(Arg::with_name("package_name")
                .required(true)
                .multiple(false)
                .index(1)
                .value_name("PACKAGE_NAME")
                .help("The name of the package")
            )
            .arg(Arg::with_name("package_version_constraint")
                .required(false)
                .multiple(false)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .help("A version constraint to search for (optional)")
            )
            .arg(Arg::with_name("dependency_type")
                .required(false)
                .multiple(true)
                .takes_value(true)
                .short('t')
                .long("type")
                .value_name("DEPENDENCY_TYPE")
                .possible_values(&[
                    IDENT_DEPENDENCY_TYPE_SYSTEM,
                    IDENT_DEPENDENCY_TYPE_SYSTEM_RUNTIME,
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .default_values(&[
                    IDENT_DEPENDENCY_TYPE_SYSTEM,
                    IDENT_DEPENDENCY_TYPE_SYSTEM_RUNTIME,
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .help("Specify which dependency types are to be printed. By default, all are checked")
            )
            .arg(Arg::with_name("list-format")
                .required(false)
                .multiple(false)
                .short('f')
                .long("format")
                .default_value(r#"{{i}} {{name}} - {{version}}: {{#if print_runtime_deps}}Runtime: {{runtime_deps}},{{/if}} {{#if print_build_deps}}Build: {{build_deps}},{{/if}} {{#if print_system_deps}}System: {{system_deps}},{{/if}} {{#if print_system_runtime_deps}}System-Runtime: {{system_runtime_deps}}{{/if}}"#)
                .help("The format to print the found packages with.")
                .long_help(&indoc::indoc!(r#"
                    The format to print the found packages with.

                    Possible tokens are:
                        i                         - Line number
                        name                      - Name of the package
                        version                   - Version of the package
                        source_url                - URL from where the source was retrieved
                        source_hash_type          - Type of hash noted in the package
                        source_hash               - Hash of the sources
                        system_deps               - System dependencies, as list
                        system_runtime_deps       - System runtime dependencies, as list
                        build_deps                - Build dependencies, as list
                        runtime_deps              - Runtime dependencies, as list
                        print_system_deps         - boolean flag to use in format string, true if user wants to see system deps
                        print_system_runtime_deps - boolean flag to use in format string, true if user wants to see system runtime deps
                        print_build_deps          - boolean flag to use in format string, true if user wants to see build deps
                        print_runtime_deps        - boolean flag to use in format string, true if user wants to see runtime deps

                    Handlebars modifiers are available.
                "#))
            )
        )

}

/// Naive check whether 's' is a 'key=value' pair or an existing environment variable
///
/// TODO: Clean up this spaghetti code
fn env_pass_validator(s: String) -> Result<(), String> {
    let v = s.split("=").collect::<Vec<_>>();

    if v.len() != 2 {
        if v.len() == 1 {
            if let Some(name) = v.get(0) {
                match std::env::var(name) {
                    Err(std::env::VarError::NotPresent) => {
                        return Err(format!("Environment variable '{}' not present", name))
                    },
                    Err(std::env::VarError::NotUnicode(_)) => {
                        return Err(format!("Environment variable '{}' not unicode", name))
                    },
                    Ok(_) => return Ok(()),
                }
            } else {
                return Err(format!("BUG")) // TODO: Make nice, not runtime error
            }
        } else {
            return Err(format!("Expected a 'key=value' string, got something different: '{}'", s))
        }
    } else {
        if let Some(key) = v.get(0) {
            if key.chars().any(|c| c == ' ' || c == '\t' || c == '\n') {
                return Err(format!("Invalid characters found in key: '{}'", s))
            }
        } else {
            return Err(format!("No key found in '{}'", s))
        }

        if let Some(value) = v.get(1) {
            if value.chars().any(|c| c == ' ' || c == '\t' || c == '\n') {
                return Err(format!("Invalid characters found in value: '{}'", s))
            }
        } else {
            return Err(format!("No value found in '{}'", s))
        }
    }

    Ok(())
}

