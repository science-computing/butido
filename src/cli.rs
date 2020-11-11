use std::path::PathBuf;

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

        .arg(Arg::with_name("hide_bars")
            .required(false)
            .multiple(false)
            .long("hide-bars")
            .help("Hide all progress bars")
        )

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

        .subcommand(App::new("db")
            .about("Database CLI interface")
            .subcommand(App::new("cli")
                .about("Start a database CLI, if installed on the current host")
                .long_about(indoc::indoc!(r#"
                    Starts a database shell on the configured database using one of the following
                    programs:
                        - psql
                        - pgcli

                    if installed.
                "#))

                .arg(Arg::with_name("tool")
                    .required(false)
                    .multiple(false)
                    .long("tool")
                    .value_name("TOOL")
                    .possible_values(&["psql", "pgcli"])
                    .help("Use a specific tool")
                )
            )

            .subcommand(App::new("artifacts")
                .about("List artifacts from the DB")
                .arg(Arg::with_name("csv")
                    .required(false)
                    .multiple(false)
                    .long("csv")
                    .takes_value(false)
                    .help("Format output as CSV")
                )
            )

            .subcommand(App::new("envvars")
                .about("List envvars from the DB")
                .arg(Arg::with_name("csv")
                    .required(false)
                    .multiple(false)
                    .long("csv")
                    .takes_value(false)
                    .help("Format output as CSV")
                )
            )

            .subcommand(App::new("images")
                .about("List images from the DB")
                .arg(Arg::with_name("csv")
                    .required(false)
                    .multiple(false)
                    .long("csv")
                    .takes_value(false)
                    .help("Format output as CSV")
                )
            )
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

            .arg(Arg::with_name("staging_dir")
                .required(false)
                .multiple(false)
                .long("staging-dir")
                .takes_value(true)
                .value_name("PATH")
                .validator(dir_exists_validator)
                .help("Do not throw dice on staging directory name, but hardcode for this run.")
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
                .takes_value(true)
                .value_name("IMAGE NAME")
                .short('I')
                .long("image")
                .help("Name of the docker image to use")
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
        )
        .subcommand(App::new("versions-of")
            .alias("versions")
            .about("List the versions of a package")
            .arg(Arg::with_name("package_name")
                .required(true)
                .multiple(false)
                .index(1)
                .value_name("PACKAGE_NAME")
                .help("The name of the package")
            )
        )
        .subcommand(App::new("env-of")
            .alias("env")
            .about("Show the ENV configured for a package")
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
        )
        .subcommand(App::new("find-pkg")
            .about("Find a package by regex")
            .arg(Arg::with_name("package_name_regex")
                .required(true)
                .multiple(false)
                .index(1)
                .value_name("REGEX")
                .help("The regex to match the package name against")
            )
            .arg(Arg::with_name("terse")
                .required(false)
                .multiple(false)
                .short('t')
                .long("terse")
                .help("Do not use the fancy format, but simply <name> <version>")
            )
        )
        .subcommand(App::new("source")
            .about("Handle package sources")
            .subcommand(App::new("verify")
                .about("Hash-check all source files")
                .arg(Arg::with_name("package_name")
                    .required(false)
                    .multiple(false)
                    .index(1)
                    .value_name("PKG")
                    .help("Verify the sources of this package (optional, if left out, all packages are checked)")
                )
            )
            .subcommand(App::new("list-missing")
                .about("List packages where the source is missing")
            )
            .subcommand(App::new("url")
                .about("Show the URL of the source of a package")
                .arg(Arg::with_name("package_name")
                    .required(false)
                    .multiple(false)
                    .index(1)
                    .value_name("PKG")
                    .help("Verify the sources of this package (optional, if left out, all packages are checked)")
                )
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

fn dir_exists_validator(s: String) -> Result<(), String> {
    if PathBuf::from(&s).is_dir() {
        Ok(())
    } else {
        Err(format!("Directory does not exist: {}", s))
    }
}

