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
use std::str::FromStr;

use clap::crate_authors;
use clap::crate_version;
use clap::App;
use clap::Arg;
use clap::ArgGroup;

// Helper types to ship around stringly typed clap API.
pub const IDENT_DEPENDENCY_TYPE_BUILD: &str = "build";
pub const IDENT_DEPENDENCY_TYPE_RUNTIME: &str = "runtime";

pub fn cli<'a>() -> App<'a> {
    App::new("butido")
        .author(crate_authors!())
        .version(crate_version!())
        .about("Generic Build Orchestration System for building linux packages with docker")

        .after_help(indoc::indoc!(r#"
            The following environment variables can be passed to butido:

                RUST_LOG - to enable logging, for exact usage see the rust cookbook
        "#))

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
            .about("Override the database host")
            .long_about(indoc::indoc!(r#"
                Override the database host set via configuration.
                Can also be overriden via environment variable 'BUTIDO_DATABASE_HOST', but this setting has precedence.
            "#))
        )
        .arg(Arg::new("database_port")
            .required(false)
            .multiple(false)
            .long("db-port")
            .value_name("PORT")
            .about("Override the database port")
            .long_about(indoc::indoc!(r#"
                Override the database port set via configuration.
                Can also be overriden via environment 'BUTIDO_DATABASE_PORT', but this setting has precedence.
            "#))
        )
        .arg(Arg::new("database_user")
            .required(false)
            .multiple(false)
            .long("db-user")
            .value_name("USER")
            .about("Override the database user")
            .long_about(indoc::indoc!(r#"
                Override the database user set via configuration.
                Can also be overriden via environment 'BUTIDO_DATABASE_USER', but this setting has precedence.
            "#))
        )
        .arg(Arg::new("database_password")
            .required(false)
            .multiple(false)
            .long("db-password")
            .alias("db-pw")
            .value_name("PASSWORD")
            .about("Override the database password")
            .long_about(indoc::indoc!(r#"
                Override the database password set via configuration.
                Can also be overriden via environment 'BUTIDO_DATABASE_PASSWORD', but this setting has precedence.
            "#))
        )
        .arg(Arg::new("database_name")
            .required(false)
            .multiple(false)
            .long("db-name")
            .value_name("NAME")
            .about("Override the database name")
            .long_about(indoc::indoc!(r#"
                Override the database name set via configuration.
                Can also be overriden via environment 'BUTIDO_DATABASE_NAME', but this setting has precedence.
            "#))
        )
        .arg(Arg::new("database_connection_timeout")
            .required(false)
            .multiple(false)
            .long("db-timeout")
            .value_name("TIMEOUT")
            .about("Override the database connection timeout")
            .long_about(indoc::indoc!(r#"
                Override the database connection timeout set via configuration.
                Can also be overriden via environment 'BUTIDO_DATABASE_CONNECTION_TIMEOUT', but this setting has precedence.
            "#))
        )

        .subcommand(App::new("generate-completions")
            .version(crate_version!())
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
            .version(crate_version!())
            .about("Database CLI interface")
            .subcommand(App::new("cli")
                .version(crate_version!())
                .about("Start a database CLI, if installed on the current host")
                .long_about(indoc::indoc!(r#"
                    Starts a database shell on the configured database using one of the following programs:

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

            .subcommand(App::new("setup")
                .version(crate_version!())
                .about("Run the database setup")
                .long_about(indoc::indoc!(r#"
                    Run the database setup migrations
                "#))
            )

            .subcommand(App::new("artifacts")
                .version(crate_version!())
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
                .version(crate_version!())
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
                .version(crate_version!())
                .about("List images from the DB")
                .arg(Arg::new("csv")
                    .required(false)
                    .multiple(false)
                    .long("csv")
                    .takes_value(false)
                    .about("Format output as CSV")
                )
            )

            .subcommand(App::new("submit")
                .version(crate_version!())
                .about("Show details about one specific submit")
                .arg(Arg::new("submit")
                    .required(true)
                    .multiple(false)
                    .index(1)
                    .takes_value(true)
                    .value_name("SUBMIT")
                    .about("The Submit to show details about")
                )
            )

            .subcommand(App::new("submits")
                .version(crate_version!())
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
                .arg(Arg::new("limit")
                    .required(false)
                    .multiple(false)
                    .long("limit")
                    .takes_value(true)
                    .value_name("LIMIT")
                    .about("Only list LIMIT submits")
                )
                .arg(Arg::new("for-commit")
                    .required(true)
                    .multiple(false)
                    .long("commit")
                    .takes_value(true)
                    .value_name("HASH")
                    .about("Limit listed submits to one commit hash")
                )
            )

            .subcommand(App::new("jobs")
                .version(crate_version!())
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

                .arg(Arg::new("env_filter")
                    .required(false)
                    .multiple(false)
                    .long("env")
                    .short('E')
                    .takes_value(true)
                    .value_name("KV")
                    .about("Filter for this \"key=value\" environment variable")
                )

                .arg(Arg::new("limit")
                    .required(false)
                    .multiple(false)
                    .long("limit")
                    .short('L')
                    .takes_value(true)
                    .value_name("LIMIT")
                    .about("Only list newest LIMIT jobs instead of all")
                )

                .arg(arg_older_than_date("List only jobs older than DATE"))
                .arg(arg_newer_than_date("List only jobs newer than DATE"))

                .arg(Arg::new("endpoint")
                    .required(false)
                    .multiple(false)
                    .long("endpoint")
                    .short('e')
                    .takes_value(true)
                    .value_name("ENDPOINT")
                    .about("Only show jobs from ENDPOINT")
                )

                .arg(Arg::new("package")
                    .required(false)
                    .multiple(false)
                    .long("package")
                    .short('p')
                    .takes_value(true)
                    .value_name("PKG")
                    .about("Only show jobs for PKG")
                )

            )

            .subcommand(App::new("job")
                .version(crate_version!())
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
            .subcommand(App::new("log-of")
                .version(crate_version!())
                .about("Print log of a job, short version of 'db job --log'")
                .arg(Arg::new("job_uuid")
                    .required(true)
                    .multiple(false)
                    .index(1)
                    .takes_value(true)
                    .value_name("UUID")
                    .about("The id of the Job")
                )
            )
            .subcommand(App::new("releases")
                .version(crate_version!())
                .about("List releases")
                .arg(Arg::new("csv")
                    .required(false)
                    .multiple(false)
                    .long("csv")
                    .takes_value(false)
                    .about("Format output as CSV")
                )

                .arg(arg_older_than_date("List only releases older than DATE"))
                .arg(arg_newer_than_date("List only releases newer than DATE"))

                .arg(Arg::new("store")
                    .required(false)
                    .multiple(false)
                    .long("to")
                    .takes_value(true)
                    .value_name("STORE")
                    .about("List only releases to STORE")
                )

                .arg(Arg::new("package")
                    .required(false)
                    .multiple(false)
                    .long("package")
                    .short('p')
                    .takes_value(true)
                    .value_name("PKG")
                    .about("Only list releases for package PKG")
                )
            )
        )

        .subcommand(App::new("build")
            .version(crate_version!())
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
                .about("Skip hashsum check")
                .long_about(indoc::indoc!(r#"
                    Do not perform a hash sum check on all packages in the dependency tree before starting the build.
                "#))
            )
            .arg(Arg::new("no_lint")
                .required(false)
                .multiple(false)
                .takes_value(false)
                .long("no-lint")
                .about("Skip linting")
                .long_about(indoc::indoc!(r#"
                    Do not perform script linting before starting the build.
                "#))
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
                .about("Pass environment variable to all build jobs")
                .long_about(indoc::indoc!(r#"
                    Pass these variables to each build job.
                    This argument expects \"key=value\" or name of variable available in ENV
                "#))
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
                .about("Write log to disk as well")
                .long_about(indoc::indoc!(r#"
                    With this flag set, butido does not only write the build logs to database, but also to the configured
                    log directory.

                    The log of a build is written to `<log_dir>/<build id>.log`.
                "#))
            )
        )

        .subcommand(App::new("what-depends")
            .version(crate_version!())
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
            .version(crate_version!())
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
            .version(crate_version!())
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
            .version(crate_version!())
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

        .subcommand(App::new("find-artifact")
            .version(crate_version!())
            .about("Find artifacts for packages")
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
            .arg(Arg::new("no_script_filter")
                .long("no-script-filter")
                .short('S')
                .required(false)
                .multiple(false)
                .takes_value(false)
                .about("Don't check for script equality. Can cause unexact results.")
            )
            .arg(Arg::new("staging_dir")
                .required(false)
                .multiple(false)
                .long("staging-dir")
                .takes_value(true)
                .value_name("PATH")
                .validator(dir_exists_validator)
                .about("Also consider this staging dir when searching for artifacts")
            )
            .arg(Arg::new("env_filter")
                .required(false)
                .multiple(true)
                .long("env")
                .short('E')
                .takes_value(true)
                .value_name("KV")
                .validator(env_pass_validator)
                .about("Filter for this \"key=value\" environment variable")
            )
        )

        .subcommand(App::new("find-pkg")
            .version(crate_version!())
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
            .version(crate_version!())
            .about("Handle package sources")
            .subcommand(App::new("verify")
                .version(crate_version!())
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

                .arg(Arg::new("matching")
                    .required(false)
                    .multiple(false)
                    .long("matching")
                    .takes_value(true)
                    .value_name("REGEX")
                    .about("Verify all packages where the package name matches REGEX")
                )

                .group(ArgGroup::new("verify-one-or-many")
                    .args(&["package_name", "matching"])
                    .required(true)
                )
            )
            .subcommand(App::new("list-missing")
                .version(crate_version!())
                .about("List packages where the source is missing")
            )
            .subcommand(App::new("url")
                .version(crate_version!())
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
                .version(crate_version!())
                .about("Download the source for one or multiple packages")
                .arg(Arg::new("package_name")
                    .required(false)
                    .multiple(false)
                    .index(1)
                    .value_name("PKG")
                    .about("Download the sources of this package (optional, if left out, all packages are downloaded)")
                )
                .arg(Arg::new("package_version")
                    .required(false)
                    .multiple(false)
                    .index(2)
                    .value_name("VERSION_CONSTRAINT")
                    .about("Download the sources of this package version (optional, if left out, all packages are downloaded)")
                )
                .arg(Arg::new("force")
                    .required(false)
                    .multiple(false)
                    .long("force")
                    .about("Overwrite existing cache entry")
                )

                .arg(Arg::new("matching")
                    .required(false)
                    .multiple(false)
                    .long("matching")
                    .takes_value(true)
                    .value_name("REGEX")
                    .about("Download all packages matching a regex with their name")
                )

                .group(ArgGroup::new("download-one-or-many")
                    .args(&["package_name", "matching"])
                    .required(true)
                )
            )
            .subcommand(App::new("of")
                .version(crate_version!())
                .about("Get the pathes of the sources of a package")
                .arg(Arg::new("package_name")
                    .required(false)
                    .multiple(false)
                    .index(1)
                    .value_name("PKG")
                    .about("Get the source file pathes for this package")
                )
                .arg(Arg::new("package_version")
                    .required(false)
                    .multiple(false)
                    .index(2)
                    .value_name("VERSION")
                    .about("Get the source file pathes for the package in this version")
                )
            )
        )

        .subcommand(App::new("release")
            .version(crate_version!())
            .about("Manage artifact releases")
            .subcommand(App::new("rm")
                .version(crate_version!())
                .about("Remove release artifacts")
                .long_about(indoc::indoc!(r#"
                    Removes a released artifact from the release store and deletes the according database entry.

                    This command asks interactively whether you want to delete data.
                    This can't be turned off.
                "#))
                .arg(Arg::new("release_store_name")
                    .required(true)
                    .multiple(false)
                    .long("from")
                    .value_name("RELEASE_STORE_NAME")
                    .about("Release store name to remove release from")
                )

                .arg(Arg::new("package_name")
                    .required(false)
                    .multiple(false)
                    .index(1)
                    .value_name("PKG")
                    .about("The name of the package")
                    .requires("package_version")
                )

                .arg(Arg::new("package_version")
                    .required(false)
                    .multiple(false)
                    .index(2)
                    .value_name("VERSION")
                    .about("The exact version of the package (string match)")
                    .requires("package_name")
                )
            )

            .subcommand(App::new("new")
                .version(crate_version!())
                .about("Release artifacts")
                .arg(Arg::new("submit_uuid")
                    .required(true)
                    .multiple(false)
                    .index(1)
                    .value_name("SUBMIT")
                    .about("The submit uuid from which to release a package")
                )
                .arg(Arg::new("release_store_name")
                    .required(true)
                    .multiple(false)
                    .long("to")
                    .value_name("RELEASE_STORE_NAME")
                    .about("Release store name to release to")
                    .long_about(indoc::indoc!(r#"
                        Butido can release to different release stores, based on this CLI flag.
                        The release stores that are available must be listed in the configuration.
                    "#))
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
                .arg(Arg::new("package_do_update")
                    .required(false)
                    .multiple(false)
                    .long("update")
                    .about("Do update a package if it already exists in the release store")
                )
                .arg(Arg::new("noninteractive")
                    .required(false)
                    .multiple(false)
                    .long("non-interactive")
                    .about("Dont be interactive (only with --update at the moment)")
                    .requires("package_do_update")
                )
                .arg(Arg::new("quiet")
                    .required(false)
                    .multiple(false)
                    .long("quiet")
                    .short('q')
                    .about("Don't print pathes to released filesfiles  after releases are complete")
                )
            )

        )

        .subcommand(App::new("lint")
            .version(crate_version!())
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
            .version(crate_version!())
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

        .subcommand(App::new("metrics")
            .version(crate_version!())
            .about("Print metrics about butido")
        )

        .subcommand(App::new("endpoint")
            .version(crate_version!())
            .about("Endpoint maintentance commands")
            .arg(Arg::new("endpoint_name")
                .required(false)
                .multiple(false)
                .index(1)
                .value_name("ENDPOINT_NAME")
                .about("Endpoint to talk to, or all if not given")
            )

            .subcommand(App::new("ping")
                .version(crate_version!())
                .about("Ping the endpoint(s)")
                .arg(Arg::new("ping_n")
                    .required(false)
                    .multiple(false)
                    .long("times")
                    .short('n')
                    .value_name("N")
                    .default_value("10")
                    .about("How often to ping")
                )
                .arg(Arg::new("ping_sleep")
                    .required(false)
                    .multiple(false)
                    .long("sleep")
                    .value_name("N")
                    .default_value("1")
                    .about("How long to sleep between pings")
                )
            )
            .subcommand(App::new("stats")
                .version(crate_version!())
                .about("Get stats for the endpoint(s)")
                .arg(Arg::new("csv")
                    .required(false)
                    .multiple(false)
                    .long("csv")
                    .takes_value(false)
                    .about("Format output as CSV")
                )
            )
            .subcommand(App::new("containers")
                .version(crate_version!())
                .about("Work with the containers of the endpoint(s)")
                .subcommand(App::new("prune")
                    .version(crate_version!())
                    .about("Remove exited containers")
                    .arg(arg_older_than_date("Prune only containers older than DATE"))
                    .arg(arg_newer_than_date("Prune only containers newer than DATE"))
                )
                .subcommand(App::new("stop")
                    .version(crate_version!())
                    .about("Stop running containers")
                    .arg(arg_older_than_date("Stop only containers older than DATE"))
                    .arg(arg_newer_than_date("Stop only containers newer than DATE"))
                    .arg(Arg::new("timeout")
                        .required(false)
                        .multiple(false)
                        .long("timeout")
                        .short('t')
                        .takes_value(true)
                        .value_name("TIMEOUT")
                        .about("Timeout in seconds")
                        .validator(parse_u64)
                    )
                )
                .subcommand(App::new("list")
                    .version(crate_version!())
                    .about("List the containers and stats about them")
                    .arg(Arg::new("csv")
                        .required(false)
                        .multiple(false)
                        .long("csv")
                        .takes_value(false)
                        .about("Format output as CSV")
                    )

                    .arg(Arg::new("list_stopped")
                        .required(false)
                        .multiple(false)
                        .long("list-stopped")
                        .takes_value(false)
                        .about("List stopped containers too")
                    )

                    .arg(Arg::new("filter_image")
                        .required(false)
                        .multiple(false)
                        .long("image")
                        .takes_value(true)
                        .value_name("IMAGE")
                        .about("List only containers of IMAGE")
                    )

                    .arg(arg_older_than_date("List only containers older than DATE"))
                    .arg(arg_newer_than_date("List only containers newer than DATE"))
                )
                .subcommand(App::new("top")
                    .version(crate_version!())
                    .about("List the processes of all containers")
                    .arg(Arg::new("csv")
                        .required(false)
                        .multiple(false)
                        .long("csv")
                        .takes_value(false)
                        .about("List top output as CSV")
                    )
                    .arg(Arg::new("limit")
                        .required(false)
                        .multiple(false)
                        .long("limit")
                        .takes_value(true)
                        .value_name("LIMIT")
                        .about("Only list LIMIT processes for each container")
                        .validator(parse_usize)
                    )
                )
            )
            .subcommand(App::new("container")
                .version(crate_version!())
                .about("Work with a specific container")
                .arg(Arg::new("container_id")
                    .required(true)
                    .multiple(false)
                    .index(1)
                    .takes_value(true)
                    .value_name("CONTAINER_ID")
                    .about("Work with container CONTAINER_ID")
                )
                .subcommand(App::new("top")
                    .version(crate_version!())
                    .about("List the container processes")
                    .arg(Arg::new("csv")
                        .required(false)
                        .multiple(false)
                        .long("csv")
                        .takes_value(false)
                        .about("List top output as CSV")
                    )
                )
                .subcommand(App::new("kill")
                    .version(crate_version!())
                    .about("Kill the container")
                    .arg(Arg::new("signal")
                        .required(false)
                        .multiple(false)
                        .index(1)
                        .takes_value(true)
                        .value_name("SIGNAL")
                        .about("Kill container with this signal")
                    )
                )
                .subcommand(App::new("delete")
                    .version(crate_version!())
                    .about("Delete the container")
                )
                .subcommand(App::new("start")
                    .version(crate_version!())
                    .about("Start the container")
                )
                .subcommand(App::new("stop")
                    .version(crate_version!())
                    .about("Stop the container")
                    .arg(Arg::new("timeout")
                        .required(false)
                        .multiple(false)
                        .long("timeout")
                        .takes_value(true)
                        .value_name("DURATION")
                        .about("Timeout")
                    )
                )
                .subcommand(App::new("exec")
                    .version(crate_version!())
                    .about("Execute commands in the container")
                    .arg(Arg::new("commands")
                        .required(true)
                        .multiple(true)
                        .index(1)
                        .takes_value(true)
                        .value_name("CMD")
                        .about("Commands to execute in the container")
                        .long_about(indoc::indoc!(r#"
                            Execute a command in the container.

                            This does not handle TTY forwarding, so you cannot execute interactive commands in the container (e.g. htop).
                            For executing interactive things, you have to login to the container.
                        "#))
                    )
                )

                .subcommand(App::new("inspect")
                    .version(crate_version!())
                    .about("Display details about the container")
                    .long_about("Display details about the container. Do not assume the output format to be stable.")
                )
            )
            .subcommand(App::new("images")
                .version(crate_version!())
                .about("Query images on endpoint(s)")
                .subcommand(App::new("list")
                    .version(crate_version!())
                    .about("List images on endpoint(s)")
                    .arg(Arg::new("csv")
                        .required(false)
                        .multiple(false)
                        .long("csv")
                        .takes_value(false)
                        .about("List top output as CSV")
                    )
                )
                .subcommand(App::new("verify-present")
                    .version(crate_version!())
                    .about("Verify that all configured images are present on endpoint(s)")
                    .arg(Arg::new("csv")
                        .required(false)
                        .multiple(false)
                        .long("csv")
                        .takes_value(false)
                        .about("List top output as CSV")
                    )
                )
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
        }
        Ok((k, v)) => {
            log::debug!("Env pass valiation: '{}={}'", k, v);
            Ok(())
        }
    }
}

fn dir_exists_validator(s: &str) -> Result<(), String> {
    if PathBuf::from(&s).is_dir() {
        Ok(())
    } else {
        Err(format!("Directory does not exist: {}", s))
    }
}

fn arg_older_than_date(about: &str) -> Arg<'_> {
    Arg::new("older_than")
        .required(false)
        .multiple(false)
        .long("older-than")
        .takes_value(true)
        .value_name("DATE")
        .about(about)
        .long_about(r#"
            DATE can be a freeform date, for example '2h'
            It can also be a exact date: '2020-01-01 00:12:45'
            If the hour-minute-second part is omitted, " 00:00:00" is appended automatically.

            Supported suffixes:

                nsec, ns -- nanoseconds
                usec, us -- microseconds
                msec, ms -- milliseconds
                seconds, second, sec, s
                minutes, minute, min, m
                hours, hour, hr, h
                days, day, d
                weeks, week, w
                months, month, M -- defined as 30.44 days
                years, year, y -- defined as 365.25 days

        "#)
        .validator(parse_date_from_string)
}

fn arg_newer_than_date(about: &str) -> Arg<'_> {
    Arg::new("newer_than")
        .required(false)
        .multiple(false)
        .long("newer-than")
        .takes_value(true)
        .value_name("DATE")
        .about(about)
        .long_about(r#"
            DATE can be a freeform date, for example '2h'
            It can also be a exact date: '2020-01-01 00:12:45'
            If the hour-minute-second part is omitted, " 00:00:00" is appended automatically.

            Supported suffixes:

                nsec, ns -- nanoseconds
                usec, us -- microseconds
                msec, ms -- milliseconds
                seconds, second, sec, s
                minutes, minute, min, m
                hours, hour, hr, h
                days, day, d
                weeks, week, w
                months, month, M -- defined as 30.44 days
                years, year, y -- defined as 365.25 days

        "#)
        .validator(parse_date_from_string)
}

fn parse_date_from_string(s: &str) -> std::result::Result<(), String> {
    humantime::parse_duration(s)
        .map_err(|e| e.to_string())
        .map(|_| ())
        .or_else(|_| {
            humantime::parse_rfc3339_weak(s)
                .map_err(|e| e.to_string())
                .map(|_| ())
        })
        .or_else(|_| {
            let s = format!("{} 00:00:00", s);
            humantime::parse_rfc3339_weak(&s)
                .map_err(|e| e.to_string())
                .map(|_| ())
        })
}

fn parse_usize(s: &str) -> std::result::Result<(), String> {
    usize::from_str(s) .map_err(|e| e.to_string()).map(|_| ())
}

fn parse_u64(s: &str) -> std::result::Result<(), String> {
    u64::from_str(s).map_err(|e| e.to_string()).map(|_| ())
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
