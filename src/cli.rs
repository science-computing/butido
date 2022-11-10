//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::path::PathBuf;

use clap::crate_authors;
use clap::crate_version;
use clap::Command;
use clap::Arg;
use clap::ArgGroup;
use clap::builder::PossibleValuesParser;
use clap::builder::ValueParser;

// Helper types to ship around stringly typed clap API.
pub const IDENT_DEPENDENCY_TYPE_BUILD: &str = "build";
pub const IDENT_DEPENDENCY_TYPE_RUNTIME: &str = "runtime";

pub fn cli() -> Command {
    Command::new("butido")
        .author(crate_authors!())
        .version(crate_version!())
        .about("Generic Build Orchestration System for building linux packages with docker")

        .after_help(indoc::indoc!(r#"
            The following environment variables can be passed to butido:

                RUST_LOG - to enable logging, for exact usage see the rust cookbook
        "#))

        .arg(Arg::new("hide_bars")
            .required(false)
            .long("hide-bars")
            .help("Hide all progress bars")
        )

        .arg(Arg::new("database_host")
            .required(false)
            .num_args(1)
            .long("db-url")
            .value_name("HOST")
            .help("Override the database host")
            .long_help(indoc::indoc!(r#"
                Override the database host set via configuration.
                Can also be overriden via environment variable 'BUTIDO_DATABASE_HOST', but this setting has precedence.
            "#))
        )
        .arg(Arg::new("database_port")
            .required(false)
            .num_args(1)
            .long("db-port")
            .value_name("PORT")
            .help("Override the database port")
            .long_help(indoc::indoc!(r#"
                Override the database port set via configuration.
                Can also be overriden via environment 'BUTIDO_DATABASE_PORT', but this setting has precedence.
            "#))
        )
        .arg(Arg::new("database_user")
            .required(false)
            .num_args(1)
            .long("db-user")
            .value_name("USER")
            .help("Override the database user")
            .long_help(indoc::indoc!(r#"
                Override the database user set via configuration.
                Can also be overriden via environment 'BUTIDO_DATABASE_USER', but this setting has precedence.
            "#))
        )
        .arg(Arg::new("database_password")
            .required(false)
            .num_args(1)
            .long("db-password")
            .alias("db-pw")
            .value_name("PASSWORD")
            .help("Override the database password")
            .long_help(indoc::indoc!(r#"
                Override the database password set via configuration.
                Can also be overriden via environment 'BUTIDO_DATABASE_PASSWORD', but this setting has precedence.
            "#))
        )
        .arg(Arg::new("database_name")
            .required(false)
            .num_args(1)
            .long("db-name")
            .value_name("NAME")
            .help("Override the database name")
            .long_help(indoc::indoc!(r#"
                Override the database name set via configuration.
                Can also be overriden via environment 'BUTIDO_DATABASE_NAME', but this setting has precedence.
            "#))
        )
        .arg(Arg::new("database_connection_timeout")
            .required(false)
            .num_args(1)
            .long("db-timeout")
            .value_name("TIMEOUT")
            .help("Override the database connection timeout")
            .long_help(indoc::indoc!(r#"
                Override the database connection timeout set via configuration.
                Can also be overriden via environment 'BUTIDO_DATABASE_CONNECTION_TIMEOUT', but this setting has precedence.
            "#))
        )

        .subcommand(Command::new("generate-completions")
            .version(crate_version!())
            .about("Generate and print commandline completions")
            .arg(Arg::new("shell")
                .value_parser(PossibleValuesParser::new(["bash", "elvish", "fish", "zsh"]))
                .default_value("bash")
                .required(true)
                .num_args(1)
                .help("Shell to generate completions for")
            )
        )

        .subcommand(Command::new("db")
            .version(crate_version!())
            .about("Database CLI interface")
            .subcommand(Command::new("cli")
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
                    .num_args(1)
                    .long("tool")
                    .value_name("TOOL")
                    .value_parser(PossibleValuesParser::new(["psql", "pgcli"]))
                    .help("Use a specific tool")
                )
            )

            .subcommand(Command::new("setup")
                .version(crate_version!())
                .about("Run the database setup")
                .long_about(indoc::indoc!(r#"
                    Run the database setup migrations
                "#))
            )

            .subcommand(Command::new("artifacts")
                .version(crate_version!())
                .about("List artifacts from the DB")
                .arg(Arg::new("csv")
                    .required(false)
                    .num_args(1)
                    .long("csv")
                    .help("Format output as CSV")
                )
                .arg(Arg::new("job_uuid")
                    .required(false)
                    .long("job")
                    .short('J')
                    .num_args(1)
                    .value_name("JOB UUID")
                    .value_parser(clap::value_parser!(uuid::Uuid))
                    .help("Print only artifacts for a certain job")
                )
            )

            .subcommand(Command::new("envvars")
                .version(crate_version!())
                .about("List envvars from the DB")
                .arg(Arg::new("csv")
                    .required(false)
                    .long("csv")
                    .num_args(0)
                    .help("Format output as CSV")
                )
            )

            .subcommand(Command::new("images")
                .version(crate_version!())
                .about("List images from the DB")
                .arg(Arg::new("csv")
                    .required(false)
                    .long("csv")
                    .num_args(0)
                    .help("Format output as CSV")
                )
            )

            .subcommand(Command::new("submit")
                .version(crate_version!())
                .about("Show details about one specific submit")
                .arg(Arg::new("submit")
                    .required(true)
                    .num_args(1)
                    .index(1)
                    .value_name("SUBMIT")
                    .value_parser(clap::value_parser!(uuid::Uuid))
                    .help("The Submit to show details about")
                )
            )

            .subcommand(Command::new("submits")
                .version(crate_version!())
                .about("List submits from the DB")
                .arg(Arg::new("csv")
                    .required(false)
                    .long("csv")
                    .num_args(0)
                    .help("Format output as CSV")
                )
                .arg(Arg::new("with_pkg")
                    .required(false)
                    .long("with-pkg")
                    .num_args(1)
                    .value_name("PKG")
                    .help("Only list submits that contained package PKG")
                    .conflicts_with("for_pkg")
                )
                .arg(Arg::new("for_pkg")
                    .required(false)
                    .long("for-pkg")
                    .num_args(1)
                    .value_name("PKG")
                    .help("Only list submits that had the root package PKG")
                    .conflicts_with("with_pkg")
                )
                .arg(Arg::new("limit")
                    .required(false)
                    .long("limit")
                    .num_args(1)
                    .value_name("LIMIT")
                    .value_parser(clap::value_parser!(i64))
                    .help("Only list LIMIT submits")
                )
                .arg(Arg::new("for-commit")
                    .required(false)
                    .long("commit")
                    .num_args(1)
                    .value_name("HASH")
                    .help("Limit listed submits to one commit hash")
                )
                .arg(Arg::new("image")
                    .required(false)
                    .long("image")
                    .num_args(1)
                    .value_name("IMAGE")
                    .help("Limit listed submits to submits on IMAGE")
                )
            )

            .subcommand(Command::new("jobs")
                .version(crate_version!())
                .about("List jobs from the DB")
                .arg(Arg::new("csv")
                    .required(false)
                    .long("csv")
                    .num_args(0)
                    .help("Format output as CSV")
                )

                .arg(Arg::new("submit_uuid")
                    .required(false)
                    .long("of-submit")
                    .short('S')
                    .num_args(1)
                    .value_name("UUID")
                    .value_parser(clap::value_parser!(uuid::Uuid))
                    .help("Only list jobs of a certain submit")
                )

                .arg(Arg::new("env_filter")
                    .required(false)
                    .long("env")
                    .short('E')
                    .num_args(1)
                    .value_name("KV")
                    .help("Filter for this \"key=value\" environment variable")
                )

                .arg(Arg::new("limit")
                    .required(false)
                    .long("limit")
                    .short('L')
                    .num_args(1)
                    .value_name("LIMIT")
                    .value_parser(clap::value_parser!(usize))
                    .help("Only list newest LIMIT jobs instead of all")
                )

                .arg(arg_older_than_date("List only jobs older than DATE"))
                .arg(arg_newer_than_date("List only jobs newer than DATE"))

                .arg(Arg::new("endpoint")
                    .required(false)
                    .long("endpoint")
                    .short('e')
                    .num_args(1)
                    .value_name("ENDPOINT")
                    .help("Only show jobs from ENDPOINT")
                )

                .arg(Arg::new("package")
                    .required(false)
                    .long("package")
                    .short('p')
                    .num_args(1)
                    .value_name("PKG")
                    .help("Only show jobs for PKG")
                )

            )

            .subcommand(Command::new("job")
                .version(crate_version!())
                .about("Show a specific job from the DB")
                .arg(Arg::new("csv")
                    .required(false)
                    .long("csv")
                    .num_args(0)
                    .help("Format output as CSV")
                )

                .arg(Arg::new("job_uuid")
                    .required(true)
                    .num_args(1)
                    .index(1)
                    .value_name("UUID")
                    .help("The job to show")
                )

                .arg(Arg::new("show_log")
                    .required(false)
                    .num_args(0)
                    .long("log")
                    .short('L')
                    .help("Show the log")
                )

                .arg(Arg::new("show_script")
                    .required(false)
                    .num_args(0)
                    .long("script")
                    .short('s')
                    .help("Show the script")
                )

                .arg(Arg::new("show_env")
                    .required(false)
                    .num_args(0)
                    .long("env")
                    .short('E')
                    .help("Show the environment of the job")
                )

                .arg(script_arg_line_numbers())
                .arg(script_arg_no_line_numbers())
                .arg(script_arg_highlight())
                .arg(script_arg_no_highlight())
            )
            .subcommand(Command::new("log-of")
                .version(crate_version!())
                .about("Print log of a job, short version of 'db job --log'")
                .arg(Arg::new("job_uuid")
                    .required(true)
                    .num_args(1)
                    .index(1)
                    .value_name("UUID")
                    .help("The id of the Job")
                )
            )
            .subcommand(Command::new("releases")
                .version(crate_version!())
                .about("List releases")
                .arg(Arg::new("csv")
                    .required(false)
                    .long("csv")
                    .num_args(0)
                    .help("Format output as CSV")
                )

                .arg(arg_older_than_date("List only releases older than DATE"))
                .arg(arg_newer_than_date("List only releases newer than DATE"))

                .arg(Arg::new("store")
                    .required(false)
                    .long("to")
                    .num_args(1)
                    .value_name("STORE")
                    .help("List only releases to STORE")
                )

                .arg(Arg::new("package")
                    .required(false)
                    .long("package")
                    .short('p')
                    .num_args(1)
                    .value_name("PKG")
                    .help("Only list releases for package PKG")
                )
            )
        )

        .subcommand(Command::new("build")
            .version(crate_version!())
            .about("Build packages in containers")

            .arg(Arg::new("package_name")
                .required(true)
                .num_args(1)
                .index(1)
                .value_name("NAME")
            )
            .arg(Arg::new("package_version")
                .required(false)
                .num_args(1)
                .index(2)
                .value_name("VERSION")
                .help("Exact package version to build (string match)")
            )

            .arg(Arg::new("no_verification")
                .required(false)
                .num_args(0)
                .long("no-verify")
                .help("Skip hashsum check")
                .long_help(indoc::indoc!(r#"
                    Do not perform a hash sum check on all packages in the dependency tree before starting the build.
                "#))
            )
            .arg(Arg::new("no_lint")
                .required(false)
                .num_args(0)
                .long("no-lint")
                .help("Skip linting")
                .long_help(indoc::indoc!(r#"
                    Do not perform script linting before starting the build.
                "#))
            )

            .arg(Arg::new("staging_dir")
                .required(false)
                .long("staging-dir")
                .num_args(1)
                .value_name("PATH")
                .value_parser(ValueParser::new(dir_exists_validator))
                .help("Do not throw dice on staging directory name, but hardcode for this run.")
            )

            .arg(Arg::new("shebang")
                .required(false)
                .long("shebang")
                .num_args(1)
                .value_name("BANG")
                .help("Overwrite the configured shebang line")
            )

            .arg(Arg::new("env")
                .required(false)
                .num_args(0)
                .short('E')
                .long("env")
                .value_parser(ValueParser::new(env_pass_validator))
                .help("Pass environment variable to all build jobs")
                .long_help(indoc::indoc!(r#"
                    Pass these variables to each build job.
                    This argument expects \"key=value\" or name of variable available in ENV
                "#))
            )

            .arg(Arg::new("image")
                .required(true)
                .num_args(1)
                .value_name("IMAGE NAME")
                .short('I')
                .long("image")
                .help("Name of the docker image to use")
            )

            .arg(Arg::new("write-log-file")
                .required(false)
                .num_args(0)
                .long("write-log")
                .short('L')
                .help("Write log to disk as well")
                .long_help(indoc::indoc!(r#"
                    With this flag set, butido does not only write the build logs to database, but also to the configured
                    log directory.

                    The log of a build is written to `<log_dir>/<build id>.log`.
                "#))
            )
        )

        .subcommand(Command::new("what-depends")
            .version(crate_version!())
            .about("List all packages that depend on a specific package")
            .arg(Arg::new("package_name")
                .required(true)
                .num_args(1)
                .index(1)
                .help("The name of the package")
            )
            .arg(Arg::new("dependency_type")
                .required(false)
                .num_args(1)
                .short('t')
                .long("type")
                .value_name("DEPENDENCY_TYPE")
                .value_parser(PossibleValuesParser::new([
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ]))
                .default_values([
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .help("Specify which dependency types are to be checked. By default, all are checked")
            )
        )
        .subcommand(Command::new("dependencies-of")
            .version(crate_version!())
            .alias("depsof")
            .about("List the depenendcies of a package")
            .arg(Arg::new("package_name")
                .required(true)
                .num_args(1)
                .index(1)
                .value_name("PACKAGE_NAME")
                .help("The name of the package")
            )
            .arg(Arg::new("package_version_constraint")
                .required(false)
                .num_args(1)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .help("A version constraint to search for (optional), E.G. '=1.0.0'")
            )
            .arg(Arg::new("dependency_type")
                .required(false)
                .num_args(1..)
                .short('t')
                .long("type")
                .value_name("DEPENDENCY_TYPE")
                .value_parser(PossibleValuesParser::new([
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ]))
                .default_values([
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .help("Specify which dependency types are to be printed. By default, all are checked")
            )
        )
        .subcommand(Command::new("versions-of")
            .version(crate_version!())
            .alias("versions")
            .about("List the versions of a package")
            .arg(Arg::new("package_name")
                .required(true)
                .num_args(1)
                .index(1)
                .value_name("PACKAGE_NAME")
                .help("The name of the package")
            )
        )
        .subcommand(Command::new("env-of")
            .version(crate_version!())
            .alias("env")
            .about("Show the ENV configured for a package")
            .arg(Arg::new("package_name")
                .required(true)
                .num_args(1)
                .index(1)
                .value_name("PACKAGE_NAME")
                .help("The name of the package")
            )
            .arg(Arg::new("package_version_constraint")
                .required(true)
                .num_args(1)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .help("A version constraint to search for (optional), E.G. '=1.0.0'")
            )
        )

        .subcommand(Command::new("find-artifact")
            .version(crate_version!())
            .about("Find artifacts for packages")
            .arg(Arg::new("package_name_regex")
                .required(true)
                .num_args(1)
                .index(1)
                .value_name("REGEX")
                .help("The regex to match the package name against")
            )
            .arg(Arg::new("package_version_constraint")
                .required(false)
                .num_args(1)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .help("A version constraint to search for (optional), E.G. '=1.0.0'")
            )
            .arg(Arg::new("no_script_filter")
                .long("no-script-filter")
                .short('S')
                .required(false)
                .num_args(0)
                .help("Don't check for script equality. Can cause unexact results.")
            )
            .arg(Arg::new("staging_dir")
                .required(false)
                .num_args(1)
                .long("staging-dir")
                .value_name("PATH")
                .value_parser(ValueParser::new(dir_exists_validator))
                .help("Also consider this staging dir when searching for artifacts")
            )
            .arg(Arg::new("env_filter")
                .required(false)
                .num_args(1..)
                .long("env")
                .short('E')
                .value_name("KV")
                .value_parser(ValueParser::new(env_pass_validator))
                .help("Filter for this \"key=value\" environment variable")
            )
            .arg(Arg::new("image")
                .required(false)
                .num_args(1)
                .long("image")
                .short('I')
                .value_name("IMAGE")
                .help("Only list artifacts that were built on IMAGE")
            )
        )

        .subcommand(Command::new("find-pkg")
            .version(crate_version!())
            .about("Find a package by regex")
            .arg(Arg::new("package_name_regex")
                .required(true)
                .num_args(1)
                .index(1)
                .value_name("REGEX")
                .help("The regex to match the package name against")
            )
            .arg(Arg::new("package_version_constraint")
                .required(false)
                .num_args(1)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .help("A version constraint to search for (optional), E.G. '=1.0.0'")
            )

            .arg(Arg::new("terse")
                .required(false)
                .num_args(1)
                .long("terse")
                .short('t')
                .help("Do not use the fancy format, but simply <name> <version>")
            )

            .arg(Arg::new("show_all")
                .required(false)
                .num_args(1)
                .long("all")
                .short('A')
                .help("Same as: -SDpEFPs --denied-images --allowed-images (all flags enabled)")
            )

            .arg(Arg::new("show_sources")
                .required(false)
                .num_args(1)
                .long("source")
                .alias("sources")
                .short('S')
                .help("Show the sources of the package")
            )

            .arg(Arg::new("show_dependencies")
                .required(false)
                .num_args(1)
                .long("dependencies")
                .alias("deps")
                .short('D')
                .help("Show the dependencies of the package")
            )
            .arg(Arg::new("dependency_type")
                .required(false)
                .num_args(1..)
                .long("dependency-type")
                .value_name("DEPENDENCY_TYPE")
                .value_parser(PossibleValuesParser::new([
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ]))
                .default_values([
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .help("Specify which dependency types are to print.")
            )

            .arg(Arg::new("show_patches")
                .required(false)
                .num_args(1)
                .long("patches")
                .short('p')
                .help("Show the patches of the package")
            )

            .arg(Arg::new("show_env")
                .required(false)
                .num_args(1)
                .long("env")
                .short('E')
                .help("Show the environment of the package")
            )

            .arg(Arg::new("show_flags")
                .required(false)
                .num_args(1)
                .long("flags")
                .short('F')
                .help("Show the flags of the package")
            )

            .arg(Arg::new("show_allowed_images")
                .required(false)
                .num_args(1)
                .long("allowed-images")
                .help("Show the images on which the package is only allowed to be built")
            )

            .arg(Arg::new("show_denied_images")
                .required(false)
                .num_args(1)
                .long("denied-images")
                .help("Show the images on which the package is not allowed to be built")
            )

            .arg(Arg::new("show_phases")
                .required(false)
                .num_args(1)
                .long("phases")
                .short('P')
                .help("Show the phases of the script of the package")
            )

            .arg(Arg::new("show_script")
                .required(false)
                .num_args(1)
                .long("script")
                .short('s')
                .help("Show the script of the package")
            )
            .arg(script_arg_line_numbers())
            .arg(script_arg_no_line_numbers())
            .arg(script_arg_highlight())
            .arg(script_arg_no_highlight())

        )
        .subcommand(Command::new("source")
            .version(crate_version!())
            .about("Handle package sources")
            .subcommand(Command::new("verify")
                .version(crate_version!())
                .about("Hash-check all source files")
                .arg(Arg::new("package_name")
                    .required(false)
                    .num_args(1)
                    .index(1)
                    .value_name("PKG")
                    .help("Verify the sources of this package (optional, if left out, all packages are checked)")
                )
                .arg(Arg::new("package_version")
                    .required(false)
                    .num_args(1)
                    .index(2)
                    .value_name("VERSION")
                    .help("Verify the sources of this package version (optional, if left out, all packages are checked)")
                )

                .arg(Arg::new("matching")
                    .required(false)
                    .num_args(1)
                    .long("matching")
                    .value_name("REGEX")
                    .help("Verify all packages where the package name matches REGEX")
                )

                .group(ArgGroup::new("verify-one-or-many")
                    .args(["package_name", "matching"])
                    .required(true)
                )
            )
            .subcommand(Command::new("list-missing")
                .version(crate_version!())
                .about("List packages where the source is missing")
            )
            .subcommand(Command::new("url")
                .version(crate_version!())
                .about("Show the URL of the source of a package")
                .arg(Arg::new("package_name")
                    .required(false)
                    .num_args(1)
                    .index(1)
                    .value_name("PKG")
                    .help("Verify the sources of this package (optional, if left out, all packages are checked)")
                )
                .arg(Arg::new("package_version")
                    .required(false)
                    .num_args(1)
                    .index(2)
                    .value_name("VERSION")
                    .help("Verify the sources of this package version (optional, if left out, all packages are checked)")
                )
            )
            .subcommand(Command::new("download")
                .version(crate_version!())
                .about("Download the source for one or multiple_values packages")
                .arg(Arg::new("package_name")
                    .required(false)
                    .num_args(1)
                    .index(1)
                    .value_name("PKG")
                    .help("Download the sources of this package")
                )
                .arg(Arg::new("package_version")
                    .required(false)
                    .num_args(1)
                    .index(2)
                    .value_name("VERSION_CONSTRAINT")
                    .help("Download the sources of this package version (optional, if left out, all packages are downloaded)")
                )
                .arg(Arg::new("force")
                    .required(false)
                    .num_args(1)
                    .long("force")
                    .help("Overwrite existing cache entry")
                )

                .arg(Arg::new("matching")
                    .required(false)
                    .num_args(1)
                    .long("matching")
                    .value_name("REGEX")
                    .help("Download all packages matching a regex with their name")
                )

                .group(ArgGroup::new("download-one-or-many")
                    .args(["package_name", "matching"])
                    .required(true)
                )

                .arg(Arg::new("timeout")
                    .required(false)
                    .num_args(1)
                    .long("timeout")
                    .value_name("TIMEOUT")
                    .value_parser(clap::value_parser!(u64))
                    .help("Set timeout for download in seconds")
                )
            )
            .subcommand(Command::new("of")
                .version(crate_version!())
                .about("Get the pathes of the sources of a package")
                .arg(Arg::new("package_name")
                    .required(false)
                    .num_args(1)
                    .index(1)
                    .value_name("PKG")
                    .help("Get the source file pathes for this package")
                )
                .arg(Arg::new("package_version")
                    .required(false)
                    .num_args(1)
                    .index(2)
                    .value_name("VERSION")
                    .help("Get the source file pathes for the package in this version")
                )
            )
        )

        .subcommand(Command::new("release")
            .version(crate_version!())
            .about("Manage artifact releases")
            .subcommand(Command::new("rm")
                .version(crate_version!())
                .about("Remove release artifacts")
                .long_about(indoc::indoc!(r#"
                    Removes a released artifact from the release store and deletes the according database entry.

                    This command asks interactively whether you want to delete data.
                    This can't be turned off.
                "#))
                .arg(Arg::new("release_store_name")
                    .required(true)
                    .num_args(1)
                    .long("from")
                    .value_name("RELEASE_STORE_NAME")
                    .help("Release store name to remove release from")
                )

                .arg(Arg::new("package_name")
                    .required(false)
                    .num_args(1)
                    .index(1)
                    .value_name("PKG")
                    .help("The name of the package")
                    .requires("package_version")
                )

                .arg(Arg::new("package_version")
                    .required(false)
                    .num_args(1)
                    .index(2)
                    .value_name("VERSION")
                    .help("The exact version of the package (string match)")
                    .requires("package_name")
                )
            )

            .subcommand(Command::new("new")
                .version(crate_version!())
                .about("Release artifacts")
                .arg(Arg::new("submit_uuid")
                    .required(true)
                    .num_args(1)
                    .index(1)
                    .value_name("SUBMIT")
                    .value_parser(clap::value_parser!(uuid::Uuid))
                    .help("The submit uuid from which to release a package")
                )
                .arg(Arg::new("release_store_name")
                    .required(true)
                    .num_args(1)
                    .long("to")
                    .value_name("RELEASE_STORE_NAME")
                    .help("Release store name to release to")
                    .long_help(indoc::indoc!(r#"
                        Butido can release to different release stores, based on this CLI flag.
                        The release stores that are available must be listed in the configuration.
                    "#))
                )
                .arg(Arg::new("package_name")
                    .required(false)
                    .num_args(1)
                    .index(2)
                    .value_name("PKG")
                    .help("The name of the package")
                    .conflicts_with("all-packages")
                )
                .arg(Arg::new("all-packages")
                    .required(false)
                    .num_args(1)
                    .long("all")
                    .help("Release all packages")
                    .conflicts_with("package_name")
                )
                .group(ArgGroup::new("package")
                    .args(["package_name", "all-packages"])
                    .required(true) // one of these is required
                )
                .arg(Arg::new("package_version")
                    .required(false)
                    .num_args(1)
                    .index(3)
                    .value_name("VERSION")
                    .help("The exact version of the package (string match)")
                )
                .arg(Arg::new("package_do_update")
                    .required(false)
                    .num_args(1)
                    .long("update")
                    .help("Do update a package if it already exists in the release store")
                )
                .arg(Arg::new("noninteractive")
                    .required(false)
                    .num_args(1)
                    .long("non-interactive")
                    .help("Dont be interactive (only with --update at the moment)")
                    .requires("package_do_update")
                )
                .arg(Arg::new("quiet")
                    .required(false)
                    .num_args(1)
                    .long("quiet")
                    .short('q')
                    .help("Don't print pathes to released filesfiles  after releases are complete")
                )
            )

        )

        .subcommand(Command::new("lint")
            .version(crate_version!())
            .about("Lint the package script of one or multiple_values packages")
            .arg(Arg::new("package_name")
                .required(false)
                .num_args(1)
                .index(1)
                .value_name("NAME")
                .help("Package name to lint (if not present, every package will be linted")
            )
            .arg(Arg::new("package_version")
                .required(false)
                .num_args(1)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .help("A version constraint to search for (optional), E.G. '=1.0.0'")
            )
        )

        .subcommand(Command::new("tree-of")
            .version(crate_version!())
            .about("Print the dependency tree of one or multiple_values packages")
            .arg(Arg::new("package_name")
                .required(true)
                .num_args(1)
                .index(1)
                .value_name("NAME")
                .help("Package name to lint (if not present, every package will be linted")
            )
            .arg(Arg::new("package_version")
                .required(false)
                .num_args(1)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .help("A version constraint to search for (optional), E.G. '=1.0.0'")
            )
            .arg(Arg::new("image")
                .required(false)
                .num_args(1)
                .value_name("IMAGE NAME")
                .short('I')
                .long("image")
                .help("Name of the docker image to use")
                .long_help(indoc::indoc!(r#"
                    Name of the docker image to use.

                    Required because tree might look different on different images because of
                    conditions on dependencies.
                "#))
            )
            .arg(Arg::new("env")
                .required(false)
                .num_args(1)
                .short('E')
                .long("env")
                .value_parser(ValueParser::new(env_pass_validator))
                .help("Additional env to be passed when building packages")
                .long_help(indoc::indoc!(r#"
                    Additional env to be passed when building packages.

                    Required because tree might look different on different images because of
                    conditions on dependencies.
                "#))
            )
        )

        .subcommand(Command::new("metrics")
            .version(crate_version!())
            .about("Print metrics about butido")
        )

        .subcommand(Command::new("endpoint")
            .version(crate_version!())
            .about("Endpoint maintentance commands")
            .arg(Arg::new("endpoint_name")
                .required(false)
                .num_args(1)
                .index(1)
                .value_name("ENDPOINT_NAME")
                .help("Endpoint to talk to, or all if not given")
            )

            .subcommand(Command::new("ping")
                .version(crate_version!())
                .about("Ping the endpoint(s)")
                .arg(Arg::new("ping_n")
                    .required(false)
                    .num_args(1)
                    .long("times")
                    .short('n')
                    .value_name("N")
                    .value_parser(clap::value_parser!(u64))
                    .default_value("10")
                    .help("How often to ping")
                )
                .arg(Arg::new("ping_sleep")
                    .required(false)
                    .num_args(1)
                    .long("sleep")
                    .value_name("N")
                    .value_parser(clap::value_parser!(u64))
                    .default_value("1")
                    .help("How long to sleep between pings")
                )
            )
            .subcommand(Command::new("stats")
                .version(crate_version!())
                .about("Get stats for the endpoint(s)")
                .arg(Arg::new("csv")
                    .required(false)
                    .num_args(1)
                    .long("csv")
                    .help("Format output as CSV")
                )
            )
            .subcommand(Command::new("containers")
                .version(crate_version!())
                .about("Work with the containers of the endpoint(s)")
                .subcommand(Command::new("prune")
                    .version(crate_version!())
                    .about("Remove exited containers")
                    .arg(arg_older_than_date("Prune only containers older than DATE"))
                    .arg(arg_newer_than_date("Prune only containers newer than DATE"))
                )
                .subcommand(Command::new("stop")
                    .version(crate_version!())
                    .about("Stop running containers")
                    .arg(arg_older_than_date("Stop only containers older than DATE"))
                    .arg(arg_newer_than_date("Stop only containers newer than DATE"))
                    .arg(Arg::new("timeout")
                        .required(false)
                        .num_args(1)
                        .long("timeout")
                        .short('t')
                        .value_name("TIMEOUT")
                        .value_parser(clap::value_parser!(u64))
                        .help("Timeout in seconds")
                    )
                )
                .subcommand(Command::new("list")
                    .version(crate_version!())
                    .about("List the containers and stats about them")
                    .arg(Arg::new("csv")
                        .required(false)
                        .long("csv")
                        .num_args(0)
                        .help("Format output as CSV")
                    )

                    .arg(Arg::new("list_stopped")
                        .required(false)
                        .long("list-stopped")
                        .num_args(0)
                        .help("List stopped containers too")
                    )

                    .arg(Arg::new("filter_image")
                        .required(false)
                        .long("image")
                        .num_args(1)
                        .value_name("IMAGE")
                        .help("List only containers of IMAGE")
                    )

                    .arg(arg_older_than_date("List only containers older than DATE"))
                    .arg(arg_newer_than_date("List only containers newer than DATE"))
                )
                .subcommand(Command::new("top")
                    .version(crate_version!())
                    .about("List the processes of all containers")
                    .arg(Arg::new("csv")
                        .required(false)
                        .long("csv")
                        .num_args(0)
                        .help("List top output as CSV")
                    )
                    .arg(Arg::new("limit")
                        .required(false)
                        .long("limit")
                        .num_args(1)
                        .value_name("LIMIT")
                        .value_parser(clap::value_parser!(usize))
                        .help("Only list LIMIT processes for each container")
                    )
                )
            )
            .subcommand(Command::new("container")
                .version(crate_version!())
                .about("Work with a specific container")
                .arg(Arg::new("container_id")
                    .required(true)
                    .num_args(1)
                    .index(1)
                    .value_name("CONTAINER_ID")
                    .help("Work with container CONTAINER_ID")
                )
                .subcommand(Command::new("top")
                    .version(crate_version!())
                    .about("List the container processes")
                    .arg(Arg::new("csv")
                        .required(false)
                        .long("csv")
                        .num_args(0)
                        .help("List top output as CSV")
                    )
                )
                .subcommand(Command::new("kill")
                    .version(crate_version!())
                    .about("Kill the container")
                    .arg(Arg::new("signal")
                        .required(false)
                        .num_args(1)
                        .index(1)
                        .value_name("SIGNAL")
                        .help("Kill container with this signal")
                    )
                )
                .subcommand(Command::new("delete")
                    .version(crate_version!())
                    .about("Delete the container")
                )
                .subcommand(Command::new("start")
                    .version(crate_version!())
                    .about("Start the container")
                )
                .subcommand(Command::new("stop")
                    .version(crate_version!())
                    .about("Stop the container")
                    .arg(Arg::new("timeout")
                        .required(false)
                        .long("timeout")
                        .num_args(1)
                        .value_name("DURATION")
                        .help("Timeout")
                    )
                )
                .subcommand(Command::new("exec")
                    .version(crate_version!())
                    .about("Execute commands in the container")
                    .arg(Arg::new("commands")
                        .required(true)
                        .num_args(1..)
                        .index(1)
                        .value_name("CMD")
                        .help("Commands to execute in the container")
                        .long_help(indoc::indoc!(r#"
                            Execute a command in the container.

                            This does not handle TTY forwarding, so you cannot execute interactive commands in the container (e.g. htop).
                            For executing interactive things, you have to login to the container.
                        "#))
                    )
                )

                .subcommand(Command::new("inspect")
                    .version(crate_version!())
                    .about("Display details about the container")
                    .long_about("Display details about the container. Do not assume the output format to be stable.")
                )
            )
            .subcommand(Command::new("images")
                .version(crate_version!())
                .about("Query images on endpoint(s)")
                .subcommand(Command::new("list")
                    .version(crate_version!())
                    .about("List images on endpoint(s)")
                    .arg(Arg::new("csv")
                        .required(false)
                        .num_args(0)
                        .long("csv")
                        .help("List top output as CSV")
                    )
                )
                .subcommand(Command::new("verify-present")
                    .version(crate_version!())
                    .about("Verify that all configured images are present on endpoint(s)")
                    .arg(Arg::new("csv")
                        .required(false)
                        .num_args(0)
                        .long("csv")
                        .help("List top output as CSV")
                    )
                )
            )
        )
}

fn script_arg_line_numbers() -> clap::Arg {
    Arg::new("script_line_numbers")
        .required(false)
        .num_args(0)
        .long("line-numbers")
        .help("Print script with line numbers (default)")
        .conflicts_with("no_script_line_numbers")
}

fn script_arg_no_line_numbers() -> clap::Arg {
    Arg::new("no_script_line_numbers")
        .required(false)
        .num_args(0)
        .long("no-line-numbers")
        .help("Print script without line numbers")
        .conflicts_with("script_line_numbers")
}

fn script_arg_highlight() -> clap::Arg {
    Arg::new("script_highlight")
        .required(false)
        .num_args(0)
        .long("highlighting")
        .alias("highlight")
        .help("Print script with highlighting (default)")
        .conflicts_with("no_script_highlight")
}

fn script_arg_no_highlight() -> clap::Arg {
    Arg::new("no_script_highlight")
        .required(false)
        .num_args(0)
        .long("no-highlighting")
        .alias("no-highlight")
        .help("Print script without highlighting")
        .conflicts_with("script_highlight")
}

/// Naive check whether 's' is a 'key=value' pair or an existing environment variable
///
/// TODO: Clean up this spaghetti code
fn env_pass_validator(s: &str) -> Result<(crate::util::EnvironmentVariableName, String), String> {
    crate::util::env::parse_to_env(s).map_err(|e| e.to_string())
}

fn dir_exists_validator(s: &str) -> Result<PathBuf, String> {
    let pb = PathBuf::from(s);
    if pb.is_dir() {
        Ok(pb)
    } else {
        Err(format!("Directory does not exist: {}", s))
    }
}

fn arg_older_than_date(about: &'static str) -> Arg {
    Arg::new("older_than")
        .required(false)
        .long("older-than")
        .num_args(1)
        .value_name("DATE")
        .help(about)
        .long_help(r#"
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
        .value_parser(ValueParser::new(parse_date_from_string))
}

fn arg_newer_than_date(about: &'static str) -> Arg {
    Arg::new("newer_than")
        .required(false)
        .long("newer-than")
        .num_args(1)
        .value_name("DATE")
        .help(about)
        .long_help(r#"
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
        .value_parser(ValueParser::new(parse_date_from_string))
}

fn parse_date_from_string(s: &str) -> std::result::Result<std::time::Duration, String> {
    humantime::parse_duration(s)
        .or_else(|_| {
            let time = humantime::parse_rfc3339_weak(s).map_err(|e| e.to_string())?;
            time.duration_since(std::time::SystemTime::now())
                .map_err(|e| e.to_string())
        })
        .or_else(|_| {
            let s = format!("{} 00:00:00", s);
            let time = humantime::parse_rfc3339_weak(&s).map_err(|e| e.to_string())?;
            time.duration_since(std::time::SystemTime::now())
                .map_err(|e| e.to_string())
        })
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
