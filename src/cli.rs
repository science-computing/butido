//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::path::PathBuf;

use clap::App;
use clap::Arg;
use clap::ArgGroup;
use clap::crate_authors;
use clap::crate_version;

// Helper types to ship around stringly typed clap API.
pub const IDENT_DEPENDENCY_TYPE_BUILD: &'static str          = "build";
pub const IDENT_DEPENDENCY_TYPE_RUNTIME: &'static str        = "runtime";

pub fn cli<'a>() -> App<'a> {
    App::new("butido")
        .author(crate_authors!())
        .version(crate_version!())
        .about("Generic Build Orchestration System for building linux packages with docker")

        .arg(Arg::new("hide_bars")
            .required(false)
            .multiple(false)
            .long("hide-bars")
            .about("Hide all progress bars")
        )

        .arg(Arg::new("database_host")
            .required(false)
            .multiple(false)
            .long("db-url")
            .value_name("HOST")
            .about("Overwrite the database host set via configuration. Can also be overriden via environment, but this setting has presendence.")
        )
        .arg(Arg::new("database_port")
            .required(false)
            .multiple(false)
            .long("db-port")
            .value_name("PORT")
            .about("Overwrite the database port set via configuration. Can also be overriden via environment, but this setting has presendence.")
        )
        .arg(Arg::new("database_user")
            .required(false)
            .multiple(false)
            .long("db-user")
            .value_name("USER")
            .about("Overwrite the database user set via configuration. Can also be overriden via environment, but this setting has presendence.")
        )
        .arg(Arg::new("database_password")
            .required(false)
            .multiple(false)
            .long("db-password")
            .alias("db-pw")
            .value_name("PASSWORD")
            .about("Overwrite the database password set via configuration. Can also be overriden via environment, but this setting has presendence.")
        )
        .arg(Arg::new("database_name")
            .required(false)
            .multiple(false)
            .long("db-name")
            .value_name("NAME")
            .about("Overwrite the database name set via configuration. Can also be overriden via environment, but this setting has presendence.")
        )

        .subcommand(App::new("generate-completions")
            .about("Generate and print commandline completions")
            .arg(Arg::new("shell")
                .possible_values(&["bash", "elvish", "fish", "zsh"])
                .default_value("bash")
                .required(true)
                .multiple(false)
                .about("Shell to generate completions for")
            )
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

                .arg(Arg::new("tool")
                    .required(false)
                    .multiple(false)
                    .long("tool")
                    .value_name("TOOL")
                    .possible_values(&["psql", "pgcli"])
                    .about("Use a specific tool")
                )
            )

            .subcommand(App::new("artifacts")
                .about("List artifacts from the DB")
                .arg(Arg::new("csv")
                    .required(false)
                    .multiple(false)
                    .long("csv")
                    .takes_value(false)
                    .about("Format output as CSV")
                )
                .arg(Arg::new("job_uuid")
                    .required(false)
                    .multiple(false)
                    .long("job")
                    .short('J')
                    .takes_value(true)
                    .value_name("JOB UUID")
                    .about("Print only artifacts for a certain job")
                )
            )

            .subcommand(App::new("envvars")
                .about("List envvars from the DB")
                .arg(Arg::new("csv")
                    .required(false)
                    .multiple(false)
                    .long("csv")
                    .takes_value(false)
                    .about("Format output as CSV")
                )
            )

            .subcommand(App::new("images")
                .about("List images from the DB")
                .arg(Arg::new("csv")
                    .required(false)
                    .multiple(false)
                    .long("csv")
                    .takes_value(false)
                    .about("Format output as CSV")
                )
            )

            .subcommand(App::new("submits")
                .about("List submits from the DB")
                .arg(Arg::new("csv")
                    .required(false)
                    .multiple(false)
                    .long("csv")
                    .takes_value(false)
                    .about("Format output as CSV")
                )
                .arg(Arg::new("with_pkg")
                    .required(false)
                    .multiple(false)
                    .long("with-pkg")
                    .takes_value(true)
                    .value_name("PKG")
                    .about("Only list submits that contained package PKG")
                    .conflicts_with("for_pkg")
                )
                .arg(Arg::new("for_pkg")
                    .required(false)
                    .multiple(false)
                    .long("for-pkg")
                    .takes_value(true)
                    .value_name("PKG")
                    .about("Only list submits that had the root package PKG")
                    .conflicts_with("with_pkg")
                )
            )

            .subcommand(App::new("jobs")
                .about("List jobs from the DB")
                .arg(Arg::new("csv")
                    .required(false)
                    .multiple(false)
                    .long("csv")
                    .takes_value(false)
                    .about("Format output as CSV")
                )

                .arg(Arg::new("submit_uuid")
                    .required(false)
                    .multiple(false)
                    .long("of-submit")
                    .short('S')
                    .takes_value(true)
                    .value_name("UUID")
                    .about("Only list jobs of a certain submit")
                )
            )

            .subcommand(App::new("job")
                .about("Show a specific job from the DB")
                .arg(Arg::new("csv")
                    .required(false)
                    .multiple(false)
                    .long("csv")
                    .takes_value(false)
                    .about("Format output as CSV")
                )

                .arg(Arg::new("job_uuid")
                    .required(true)
                    .multiple(false)
                    .index(1)
                    .takes_value(true)
                    .value_name("UUID")
                    .about("The job to show")
                )

                .arg(Arg::new("show_log")
                    .required(false)
                    .multiple(false)
                    .long("log")
                    .short('L')
                    .about("Show the log")
                )

                .arg(Arg::new("show_script")
                    .required(false)
                    .multiple(false)
                    .long("script")
                    .short('s')
                    .about("Show the script")
                )

                .arg(Arg::new("show_env")
                    .required(false)
                    .multiple(false)
                    .long("env")
                    .short('E')
                    .about("Show the environment of the job")
                )

                .arg(script_arg_line_numbers())
                .arg(script_arg_no_line_numbers())
                .arg(script_arg_highlight())
                .arg(script_arg_no_highlight())
            )
        )

        .subcommand(App::new("build")
            .about("Build packages in containers")

            .arg(Arg::new("package_name")
                .required(true)
                .multiple(false)
                .index(1)
                .value_name("NAME")
            )
            .arg(Arg::new("package_version")
                .required(false)
                .multiple(false)
                .index(2)
                .value_name("VERSION")
                .about("Exact package version to build (string match)")
            )

            .arg(Arg::new("no_verification")
                .required(false)
                .multiple(false)
                .takes_value(false)
                .long("no-verify")
                .about("Do not perform a hash sum check on all packages in the dependency tree before starting the build")
            )
            .arg(Arg::new("no_lint")
                .required(false)
                .multiple(false)
                .takes_value(false)
                .long("no-lint")
                .about("Do not perform script linting before starting the build")
            )

            .arg(Arg::new("staging_dir")
                .required(false)
                .multiple(false)
                .long("staging-dir")
                .takes_value(true)
                .value_name("PATH")
                .validator(dir_exists_validator)
                .about("Do not throw dice on staging directory name, but hardcode for this run.")
            )

            .arg(Arg::new("shebang")
                .required(false)
                .multiple(false)
                .long("shebang")
                .takes_value(true)
                .value_name("BANG")
                .about("Overwrite the configured shebang line")
            )

            .arg(Arg::new("env")
                .required(false)
                .multiple(true)
                .short('E')
                .long("env")
                .validator(env_pass_validator)
                .about("Pass these variables to each build job (expects \"key=value\" or name of variable available in ENV)")
            )

            .arg(Arg::new("image")
                .required(true)
                .multiple(false)
                .takes_value(true)
                .value_name("IMAGE NAME")
                .short('I')
                .long("image")
                .about("Name of the docker image to use")
            )

            .arg(Arg::new("write-log-file")
                .required(false)
                .multiple(false)
                .long("write-log")
                .short('L')
                .about("Write the log not only to database, but also in a plain-text-file")
            )
        )

        .subcommand(App::new("what-depends")
            .about("List all packages that depend on a specific package")
            .arg(Arg::new("package_name")
                .required(true)
                .multiple(false)
                .index(1)
                .about("The name of the package")
            )
            .arg(Arg::new("dependency_type")
                .required(false)
                .multiple(true)
                .takes_value(true)
                .short('t')
                .long("type")
                .value_name("DEPENDENCY_TYPE")
                .possible_values(&[
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .default_values(&[
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .about("Specify which dependency types are to be checked. By default, all are checked")
            )
        )
        .subcommand(App::new("dependencies-of")
            .alias("depsof")
            .about("List the depenendcies of a package")
            .arg(Arg::new("package_name")
                .required(true)
                .multiple(false)
                .index(1)
                .value_name("PACKAGE_NAME")
                .about("The name of the package")
            )
            .arg(Arg::new("package_version_constraint")
                .required(false)
                .multiple(false)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .about("A version constraint to search for (optional), E.G. '=1.0.0'")
            )
            .arg(Arg::new("dependency_type")
                .required(false)
                .multiple(true)
                .takes_value(true)
                .short('t')
                .long("type")
                .value_name("DEPENDENCY_TYPE")
                .possible_values(&[
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .default_values(&[
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .about("Specify which dependency types are to be printed. By default, all are checked")
            )
        )
        .subcommand(App::new("versions-of")
            .alias("versions")
            .about("List the versions of a package")
            .arg(Arg::new("package_name")
                .required(true)
                .multiple(false)
                .index(1)
                .value_name("PACKAGE_NAME")
                .about("The name of the package")
            )
        )
        .subcommand(App::new("env-of")
            .alias("env")
            .about("Show the ENV configured for a package")
            .arg(Arg::new("package_name")
                .required(true)
                .multiple(false)
                .index(1)
                .value_name("PACKAGE_NAME")
                .about("The name of the package")
            )
            .arg(Arg::new("package_version_constraint")
                .required(true)
                .multiple(false)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .about("A version constraint to search for (optional), E.G. '=1.0.0'")
            )
        )
        .subcommand(App::new("find-pkg")
            .about("Find a package by regex")
            .arg(Arg::new("package_name_regex")
                .required(true)
                .multiple(false)
                .index(1)
                .value_name("REGEX")
                .about("The regex to match the package name against")
            )
            .arg(Arg::new("package_version_constraint")
                .required(false)
                .multiple(false)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .about("A version constraint to search for (optional), E.G. '=1.0.0'")
            )

            .arg(Arg::new("terse")
                .required(false)
                .multiple(false)
                .long("terse")
                .short('t')
                .about("Do not use the fancy format, but simply <name> <version>")
            )

            .arg(Arg::new("show_all")
                .required(false)
                .multiple(false)
                .long("all")
                .short('A')
                .about("Same as: -SDpEFPs --denied-images --allowed-images (all flags enabled)")
            )

            .arg(Arg::new("show_sources")
                .required(false)
                .multiple(false)
                .long("source")
                .alias("sources")
                .short('S')
                .about("Show the sources of the package")
            )

            .arg(Arg::new("show_dependencies")
                .required(false)
                .multiple(false)
                .long("dependencies")
                .alias("deps")
                .short('D')
                .about("Show the dependencies of the package")
            )
            .arg(Arg::new("dependency_type")
                .required(false)
                .multiple(true)
                .takes_value(true)
                .long("dependency-type")
                .value_name("DEPENDENCY_TYPE")
                .possible_values(&[
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .default_values(&[
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .about("Specify which dependency types are to print.")
            )

            .arg(Arg::new("show_patches")
                .required(false)
                .multiple(false)
                .long("patches")
                .short('p')
                .about("Show the patches of the package")
            )

            .arg(Arg::new("show_env")
                .required(false)
                .multiple(false)
                .long("env")
                .short('E')
                .about("Show the environment of the package")
            )

            .arg(Arg::new("show_flags")
                .required(false)
                .multiple(false)
                .long("flags")
                .short('F')
                .about("Show the flags of the package")
            )

            .arg(Arg::new("show_allowed_images")
                .required(false)
                .multiple(false)
                .long("allowed-images")
                .about("Show the images on which the package is only allowed to be built")
            )

            .arg(Arg::new("show_denied_images")
                .required(false)
                .multiple(false)
                .long("denied-images")
                .about("Show the images on which the package is not allowed to be built")
            )

            .arg(Arg::new("show_phases")
                .required(false)
                .multiple(false)
                .long("phases")
                .short('P')
                .about("Show the phases of the script of the package")
            )

            .arg(Arg::new("show_script")
                .required(false)
                .multiple(false)
                .long("script")
                .short('s')
                .about("Show the script of the package")
            )
            .arg(script_arg_line_numbers())
            .arg(script_arg_no_line_numbers())
            .arg(script_arg_highlight())
            .arg(script_arg_no_highlight())

        )
        .subcommand(App::new("source")
            .about("Handle package sources")
            .subcommand(App::new("verify")
                .about("Hash-check all source files")
                .arg(Arg::new("package_name")
                    .required(false)
                    .multiple(false)
                    .index(1)
                    .value_name("PKG")
                    .about("Verify the sources of this package (optional, if left out, all packages are checked)")
                )
                .arg(Arg::new("package_version")
                    .required(false)
                    .multiple(false)
                    .index(2)
                    .value_name("VERSION")
                    .about("Verify the sources of this package version (optional, if left out, all packages are checked)")
                )
            )
            .subcommand(App::new("list-missing")
                .about("List packages where the source is missing")
            )
            .subcommand(App::new("url")
                .about("Show the URL of the source of a package")
                .arg(Arg::new("package_name")
                    .required(false)
                    .multiple(false)
                    .index(1)
                    .value_name("PKG")
                    .about("Verify the sources of this package (optional, if left out, all packages are checked)")
                )
                .arg(Arg::new("package_version")
                    .required(false)
                    .multiple(false)
                    .index(2)
                    .value_name("VERSION")
                    .about("Verify the sources of this package version (optional, if left out, all packages are checked)")
                )
            )
            .subcommand(App::new("download")
                .about("Download the source for one or multiple packages")
                .arg(Arg::new("package_name")
                    .required(false)
                    .multiple(false)
                    .index(1)
                    .value_name("PKG")
                    .about("Verify the sources of this package (optional, if left out, all packages are checked)")
                )
                .arg(Arg::new("package_version")
                    .required(false)
                    .multiple(false)
                    .index(2)
                    .value_name("VERSION")
                    .about("Verify the sources of this package version (optional, if left out, all packages are checked)")
                )
                .arg(Arg::new("force")
                    .required(false)
                    .multiple(false)
                    .long("force")
                    .about("Overwrite existing cache entry")
                )
            )
        )

        .subcommand(App::new("release")
            .about("Release artifacts")
            .arg(Arg::new("submit_uuid")
                .required(true)
                .multiple(false)
                .index(1)
                .value_name("SUBMIT")
                .about("The submit uuid from which to release a package")
            )
            .arg(Arg::new("package_name")
                .required(false)
                .multiple(false)
                .index(2)
                .value_name("PKG")
                .about("The name of the package")
                .conflicts_with("all-packages")
            )
            .arg(Arg::new("all-packages")
                .required(false)
                .multiple(false)
                .long("all")
                .about("Release all packages")
                .conflicts_with("package_name")
            )
            .group(ArgGroup::new("package")
                .args(&["package_name", "all-packages"])
                .required(true) // one of these is required
            )
            .arg(Arg::new("package_version")
                .required(false)
                .multiple(false)
                .index(3)
                .value_name("VERSION")
                .about("The exact version of the package (string match)")
            )
        )

        .subcommand(App::new("lint")
            .about("Lint the package script of one or multiple packages")
            .arg(Arg::new("package_name")
                .required(false)
                .multiple(false)
                .index(1)
                .value_name("NAME")
                .about("Package name to lint (if not present, every package will be linted")
            )
            .arg(Arg::new("package_version")
                .required(false)
                .multiple(false)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .about("A version constraint to search for (optional), E.G. '=1.0.0'")
            )
        )

        .subcommand(App::new("tree-of")
            .about("Print the dependency tree of one or multiple packages")
            .arg(Arg::new("package_name")
                .required(true)
                .multiple(false)
                .index(1)
                .value_name("NAME")
                .about("Package name to lint (if not present, every package will be linted")
            )
            .arg(Arg::new("package_version")
                .required(false)
                .multiple(false)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .about("A version constraint to search for (optional), E.G. '=1.0.0'")
            )
        )

}

fn script_arg_line_numbers<'a>() -> clap::Arg<'a> {
    Arg::new("script_line_numbers")
        .required(false)
        .multiple(false)
        .long("line-numbers")
        .about("Print script with line numbers (default)")
        .conflicts_with("no_script_line_numbers")
}

fn script_arg_no_line_numbers<'a>() -> clap::Arg<'a> {
    Arg::new("no_script_line_numbers")
        .required(false)
        .multiple(false)
        .long("no-line-numbers")
        .about("Print script without line numbers")
        .conflicts_with("script_line_numbers")
}

fn script_arg_highlight<'a>() -> clap::Arg<'a> {
    Arg::new("script_highlight")
        .required(false)
        .multiple(false)
        .long("highlighting")
        .alias("highlight")
        .about("Print script with highlighting (default)")
        .conflicts_with("no_script_highlight")
}

fn script_arg_no_highlight<'a>() -> clap::Arg<'a> {
    Arg::new("no_script_highlight")
        .required(false)
        .multiple(false)
        .long("no-highlighting")
        .alias("no-highlight")
        .about("Print script without highlighting")
        .conflicts_with("script_highlight")
}

/// Naive check whether 's' is a 'key=value' pair or an existing environment variable
///
/// TODO: Clean up this spaghetti code
fn env_pass_validator(s: &str) -> Result<(), String> {
    use crate::util::parser::*;
    let parser = {
        let key = (letters() + ((letters() | numbers() | under()).repeat(0..)))
            .collect()
            .convert(|b| String::from_utf8(b.to_vec()));

        let val = nonempty_string_with_optional_quotes()
            .collect()
            .convert(|b| String::from_utf8(b.to_vec()));

        (key + equal() + val).map(|((k, _), v)| (k, v))
    };

    match parser.parse(s.as_bytes()).map_err(|e| e.to_string()) {
        Err(s) => {
            log::error!("Error during validation: '{}' is not a key-value pair", s);
            Err(s)
        },
        Ok((k, v)) => {
            log::debug!("Env pass valiation: '{}={}'", k, v);
            Ok(())
        },
    }
}

fn dir_exists_validator(s: &str) -> Result<(), String> {
    if PathBuf::from(&s).is_dir() {
        Ok(())
    } else {
        Err(format!("Directory does not exist: {}", s))
    }
}

#[cfg(test)]
mod tests {
    use super::env_pass_validator;

    #[test]
    fn test_env_pass_validator_1() {
        assert!(env_pass_validator("foo=\"bar\"").is_ok());
    }

    #[test]
    fn test_env_pass_validator_2() {
        assert!(env_pass_validator("foo=bar").is_ok());
    }

    #[test]
    fn test_env_pass_validator_3() {
        assert!(env_pass_validator("foo=\"1\"").is_ok());
    }

    #[test]
    fn test_env_pass_validator_4() {
        assert!(env_pass_validator("foo=1").is_ok());
    }

    #[test]
    fn test_env_pass_validator_5() {
        assert!(env_pass_validator("FOO=\"bar\"").is_ok());
    }

    #[test]
    fn test_env_pass_validator_6() {
        assert!(env_pass_validator("FOO=bar").is_ok());
    }

    #[test]
    fn test_env_pass_validator_7() {
        assert!(env_pass_validator("FOO=\"1\"").is_ok());
    }

    #[test]
    fn test_env_pass_validator_8() {
        assert!(env_pass_validator("FOO=1").is_ok());
    }

    #[test]
    fn test_env_pass_validator_9() {
        assert!(env_pass_validator("1=1").is_err());
    }

    #[test]
    fn test_env_pass_validator_10() {
        assert!(env_pass_validator("=").is_err());
    }

    #[test]
    fn test_env_pass_validator_11() {
        assert!(env_pass_validator("a=").is_err());
    }

    #[test]
    fn test_env_pass_validator_12() {
        assert!(env_pass_validator("=a").is_err());
    }

    #[test]
    fn test_env_pass_validator_13() {
        assert!(env_pass_validator("a").is_err());
    }

    #[test]
    fn test_env_pass_validator_14() {
        assert!(env_pass_validator("avjasva").is_err());
    }

    #[test]
    fn test_env_pass_validator_15() {
        assert!(env_pass_validator("123").is_err());
    }
}

