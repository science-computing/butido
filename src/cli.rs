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
use clap::Arg;
use clap::ArgAction;
use clap::ArgGroup;
use clap::Command;

use tracing::{debug, error};

// Helper types to ship around stringly typed clap API.
pub const IDENT_DEPENDENCY_TYPE_BUILD: &str = "build";
pub const IDENT_DEPENDENCY_TYPE_RUNTIME: &str = "runtime";

pub fn cli() -> Command {
    let releases_list_command = Command::new("releases")
        .about("List releases")
        .arg(
            Arg::new("csv")
                .action(ArgAction::SetTrue)
                .required(false)
                .long("csv")
                .help("Format output as CSV"),
        )
        .arg(arg_older_than_date("List only releases older than DATE"))
        .arg(arg_newer_than_date("List only releases newer than DATE"))
        .arg(
            Arg::new("store")
                .required(false)
                .long("to")
                .value_name("STORE")
                .help("List only releases to STORE"),
        )
        .arg(
            Arg::new("package")
                .required(false)
                .long("package")
                .short('p')
                .value_name("PKG")
                .help("Only list releases for package PKG"),
        )
        .arg(
            Arg::new("limit")
                .required(false)
                .long("limit")
                .short('L')
                .value_name("LIMIT")
                .help("List newest LIMIT releases (0=unlimited)")
                .value_parser(clap::value_parser!(usize)),
        );

    Command::new("butido")
        .author(crate_authors!())
        .disable_version_flag(true)
        .about("Generic Build Orchestration System for building Linux packages with Docker")
        .after_help(indoc::indoc!(r#"
            The following environment variables can be passed to butido:

                RUST_LOG - to enable logging, for exact usage see the Rust cookbook
        "#))

        .arg(Arg::new("version")
            .action(ArgAction::SetTrue)
            .required(false)
            .short('V')
            .long("version")
            .help("Detailed version output with build information")
        )

        .arg(Arg::new("tracing-chrome")
            .action(ArgAction::SetTrue)
            .required(false)
            .long("tracing-chrome")
            .help("Generate a Chrome compatible trace file (trace-*.json)")
        )

        .arg(Arg::new("hide_bars")
            .action(ArgAction::SetTrue)
            .required(false)
            .long("hide-bars")
            .help("Hide all progress bars")
        )

        .arg(Arg::new("database_host")
            .required(false)
            .long("db-url")
            .value_name("HOST")
            .help("Override the database host")
            .long_help(indoc::indoc!(r#"
                Override the database host set via configuration.
                Can also be overridden via environment variable 'BUTIDO_DATABASE_HOST', but this setting has precedence.
            "#))
        )
        .arg(Arg::new("database_port")
            .required(false)
            .long("db-port")
            .value_name("PORT")
            .help("Override the database port")
            .long_help(indoc::indoc!(r#"
                Override the database port set via configuration.
                Can also be overridden via environment 'BUTIDO_DATABASE_PORT', but this setting has precedence.
            "#))
            .value_parser(clap::value_parser!(u16))
        )
        .arg(Arg::new("database_user")
            .required(false)
            .long("db-user")
            .value_name("USER")
            .help("Override the database user")
            .long_help(indoc::indoc!(r#"
                Override the database user set via configuration.
                Can also be overridden via environment 'BUTIDO_DATABASE_USER', but this setting has precedence.
            "#))
        )
        .arg(Arg::new("database_password")
            .required(false)
            .long("db-password")
            .alias("db-pw")
            .value_name("PASSWORD")
            .help("Override the database password")
            .long_help(indoc::indoc!(r#"
                Override the database password set via configuration.
                Can also be overridden via environment 'BUTIDO_DATABASE_PASSWORD', but this setting has precedence.
            "#))
        )
        .arg(Arg::new("database_name")
            .required(false)
            .long("db-name")
            .value_name("NAME")
            .help("Override the database name")
            .long_help(indoc::indoc!(r#"
                Override the database name set via configuration.
                Can also be overridden via environment 'BUTIDO_DATABASE_NAME', but this setting has precedence.
            "#))
        )
        .arg(Arg::new("database_connection_timeout")
            .required(false)
            .long("db-timeout")
            .value_name("TIMEOUT")
            .help("Override the database connection timeout (in seconds)")
            .long_help(indoc::indoc!(r#"
                Override the database connection timeout set via configuration.
                Can also be overridden via environment 'BUTIDO_DATABASE_CONNECTION_TIMEOUT', but this setting has precedence.
            "#))
            .value_parser(clap::value_parser!(u16))
        )

        .subcommand(Command::new("generate-completions")
            .about("Generate and print commandline completions")
            .arg(Arg::new("shell")
                .value_parser(clap::value_parser!(clap_complete::Shell))
                .default_value("bash")
                .required(false)
                .help("Shell to generate completions for")
            )
        )

        .subcommand(Command::new("db")
            .about("Database CLI interface")
            .subcommand(Command::new("cli")
                .about("Start a database CLI, if installed on the current host")
                .long_about(indoc::indoc!(r#"
                    Starts a database shell on the configured database using one of the following programs:

                        - psql
                        - pgcli

                    if installed.
                "#))

                .arg(Arg::new("tool")
                    .required(false)
                    .long("tool")
                    .value_name("TOOL")
                    .value_parser(["psql", "pgcli"])
                    .help("Use a specific tool")
                )
            )

            .subcommand(Command::new("setup")
                .about("Run the database setup")
                .long_about(indoc::indoc!(r#"
                    Run the database setup migrations
                "#))
            )

            .subcommand(Command::new("artifacts")
                .about("List artifacts from the DB")
                .arg(Arg::new("csv")
                    .action(ArgAction::SetTrue)
                    .required(false)
                    .long("csv")
                    .help("Format output as CSV")
                )
                .arg(Arg::new("job_uuid")
                    .required(false)
                    .long("job")
                    .short('J')
                    .value_name("JOB UUID")
                    .help("Print only artifacts for a certain job")
                    .value_parser(uuid::Uuid::parse_str)
                )
                .arg(Arg::new("limit")
                    .required(false)
                    .long("limit")
                    .short('L')
                    .value_name("LIMIT")
                    .help("List newest LIMIT artifacts (0=unlimited)")
                    .value_parser(clap::value_parser!(usize))
                )
            )

            .subcommand(Command::new("envvars")
                .about("List envvars from the DB")
                .arg(Arg::new("csv")
                    .action(ArgAction::SetTrue)
                    .required(false)
                    .long("csv")
                    .help("Format output as CSV")
                )
            )

            .subcommand(Command::new("images")
                .about("List images from the DB")
                .arg(Arg::new("csv")
                    .action(ArgAction::SetTrue)
                    .required(false)
                    .long("csv")
                    .help("Format output as CSV")
                )
            )

            .subcommand(Command::new("submit")
                .about("Show details about one specific submit")
                .arg(Arg::new("submit")
                    .required(true)
                    .index(1)
                    .value_name("SUBMIT")
                    .help("The Submit to show details about")
                    .value_parser(uuid::Uuid::parse_str)
                )
            )

            .subcommand(Command::new("submits")
                .about("List submits from the DB")
                .arg(Arg::new("csv")
                    .action(ArgAction::SetTrue)
                    .required(false)
                    .long("csv")
                    .help("Format output as CSV")
                )
                .arg(Arg::new("with_pkg")
                    .required(false)
                    .long("with-pkg")
                    .value_name("PKG")
                    .help("Only list submits that contained package PKG")
                    .conflicts_with("for_pkg")
                )
                .arg(Arg::new("for_pkg")
                    .required(false)
                    .long("for-pkg")
                    .value_name("PKG")
                    .help("Only list submits that had the root package PKG")
                    .conflicts_with("with_pkg")
                )
                .arg(Arg::new("limit")
                    .required(false)
                    .long("limit")
                    .short('L')
                    .value_name("LIMIT")
                    .help("List newest LIMIT submits (0=unlimited)")
                    .value_parser(clap::value_parser!(usize))
                )
                .arg(Arg::new("for-commit")
                    .required(false)
                    .long("commit")
                    .value_name("HASH")
                    .help("Limit listed submits to one commit hash")
                )
                .arg(Arg::new("image")
                    .required(false)
                    .short('I')
                    .long("image")
                    .value_name("IMAGE")
                    .help("Limit listed submits to submits on IMAGE")
                )
            )

            .subcommand(Command::new("jobs")
                .about("List jobs from the DB")
                .arg(Arg::new("csv")
                    .action(ArgAction::SetTrue)
                    .required(false)
                    .long("csv")
                    .help("Format output as CSV")
                )

                .arg(Arg::new("submit_uuid")
                    .required(false)
                    .long("of-submit")
                    .short('S')
                    .value_name("UUID")
                    .help("Only list jobs of a certain submit")
                    .value_parser(uuid::Uuid::parse_str)
                )

                .arg(Arg::new("image")
                    .required(false)
                    .value_name("IMAGE NAME")
                    .short('I')
                    .long("image")
                    .help("Only list jobs built with the Docker image IMAGE NAME")
                )

                .arg(Arg::new("env_filter")
                    .required(false)
                    .long("env")
                    .short('E')
                    .value_name("KV")
                    .help("Filter for this \"key=value\" environment variable")
                )

                .arg(Arg::new("limit")
                    .required(false)
                    .long("limit")
                    .short('L')
                    .value_name("LIMIT")
                    .help("List newest LIMIT jobs (0=unlimited)")
                    .value_parser(clap::value_parser!(usize))
                )

                .arg(arg_older_than_date("List only jobs older than DATE"))
                .arg(arg_newer_than_date("List only jobs newer than DATE"))

                .arg(Arg::new("endpoint")
                    .required(false)
                    .long("endpoint")
                    .short('e')
                    .value_name("ENDPOINT")
                    .help("Only show jobs from ENDPOINT")
                )

                .arg(Arg::new("package")
                    .required(false)
                    .long("package")
                    .short('p')
                    .value_name("PKG")
                    .help("Only show jobs for PKG")
                )

            )

            .subcommand(Command::new("job")
                .about("Show a specific job from the DB")
                .arg(Arg::new("csv")
                    .action(ArgAction::SetTrue)
                    .required(false)
                    .long("csv")
                    .help("Format output as CSV")
                )

                .arg(Arg::new("job_uuid")
                    .required(true)
                    .index(1)
                    .value_name("UUID")
                    .help("The job to show")
                    .value_parser(uuid::Uuid::parse_str)
                )

                .arg(Arg::new("show_log")
                    .action(ArgAction::SetTrue)
                    .required(false)
                    .long("log")
                    .short('L')
                    .help("Show the log")
                )

                .arg(Arg::new("show_script")
                    .action(ArgAction::SetTrue)
                    .required(false)
                    .long("script")
                    .short('s')
                    .help("Show the script")
                )

                .arg(Arg::new("show_env")
                    .action(ArgAction::SetTrue)
                    .required(false)
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
                .about("Print log of a job, short version of 'db job --log'")
                .arg(Arg::new("job_uuid")
                    .required(true)
                    .index(1)
                    .value_name("UUID")
                    .help("The id of the Job")
                    .value_parser(uuid::Uuid::parse_str)
                )
            )
            .subcommand(releases_list_command.clone())
        )

        .subcommand(Command::new("build")
            .about("Build packages in containers")

            .arg(Arg::new("package_name")
                .required(true)
                .index(1)
                .value_name("NAME")
            )
            .arg(Arg::new("package_version")
                .required(false)
                .index(2)
                .value_name("VERSION")
                .help("Exact package version to build (string match)")
            )

            .arg(Arg::new("no_verification")
                .action(ArgAction::SetTrue)
                .required(false)
                .long("no-verify")
                .help("Skip hashsum check")
                .long_help(indoc::indoc!(r#"
                    Do not perform a hash sum check on all packages in the dependency tree before starting the build.
                "#))
            )
            .arg(Arg::new("no_lint")
                .action(ArgAction::SetTrue)
                .required(false)
                .long("no-lint")
                .help("Skip linting")
                .long_help(indoc::indoc!(r#"
                    Do not perform script linting before starting the build.
                "#))
            )

            .arg(Arg::new("staging_dir")
                .required(false)
                .long("staging-dir")
                .value_name("PATH")
                .value_parser(dir_exists_validator)
                .help("Do not throw dice on staging directory name, but hardcode for this run.")
            )

            .arg(Arg::new("shebang")
                .required(false)
                .long("shebang")
                .value_name("BANG")
                .help("Overwrite the configured shebang line")
            )

            .arg(Arg::new("env")
                .required(false)
                .action(ArgAction::Append)
                .short('E')
                .long("env")
                .value_parser(env_pass_validator)
                .help("Pass environment variable to all build jobs")
                .long_help(indoc::indoc!(r#"
                    Pass these variables to each build job.
                    This argument expects \"key=value\" or name of variable available in ENV
                "#))
            )

            .arg(Arg::new("image")
                .required(true)
                .value_name("IMAGE NAME")
                .short('I')
                .long("image")
                .help("Name of the Docker image to use")
            )

            .arg(Arg::new("write-log-file")
                .action(ArgAction::SetTrue)
                .required(false)
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
            .about("List all packages that depend on a specific package")
            .arg(Arg::new("package_name")
                .required(true)
                .index(1)
                .help("The name of the package")
            )
            .arg(Arg::new("dependency_type")
                .required(false)
                .action(ArgAction::Append)
                .short('t')
                .long("type")
                .value_name("DEPENDENCY_TYPE")
                .value_parser([
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .default_values([
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .help("Specify which dependency types are to be checked. By default, all are checked")
            )
        )
        .subcommand(Command::new("dependencies-of")
            .alias("depsof")
            .about("List the dependencies of a package")
            .arg(Arg::new("package_name")
                .required(true)
                .index(1)
                .value_name("PACKAGE_NAME")
                .help("The name of the package")
            )
            .arg(Arg::new("package_version_constraint")
                .required(false)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .help("A version constraint to search for (optional), e.g., '=1.0.0'")
            )
            .arg(Arg::new("dependency_type")
                .required(false)
                .action(ArgAction::Append)
                .short('t')
                .long("type")
                .value_name("DEPENDENCY_TYPE")
                .value_parser([
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .default_values([
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .help("Specify which dependency types are to be printed. By default, all are checked")
            )
        )
        .subcommand(Command::new("versions-of")
            .alias("versions")
            .about("List the versions of a package")
            .arg(Arg::new("package_name")
                .required(true)
                .index(1)
                .value_name("PACKAGE_NAME")
                .help("The name of the package")
            )
        )
        .subcommand(Command::new("env-of")
            .alias("env")
            .about("Show the ENV configured for a package")
            .arg(Arg::new("package_name")
                .required(true)
                .index(1)
                .value_name("PACKAGE_NAME")
                .help("The name of the package")
            )
            .arg(Arg::new("package_version_constraint")
                .required(true)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .help("A version constraint to search for (optional), e.g., '=1.0.0'")
            )
        )

        .subcommand(Command::new("find-artifact")
            .about("Find artifacts for packages")
            .arg(Arg::new("package_name_regex")
                .required(true)
                .index(1)
                .value_name("REGEX")
                .help("The regex to match the package name against")
            )
            .arg(Arg::new("package_version_constraint")
                .required(false)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .help("A version constraint to search for (optional), e.g., '=1.0.0'")
            )
            .arg(Arg::new("no_script_filter")
                .action(ArgAction::SetTrue)
                .long("no-script-filter")
                .short('S')
                .required(false)
                .help("Don't check for script equality. Can cause inexact results.")
            )
            .arg(Arg::new("staging_dir")
                .required(false)
                .long("staging-dir")
                .value_name("PATH")
                .value_parser(dir_exists_validator)
                .help("Also consider this staging directory when searching for artifacts")
            )
            .arg(Arg::new("env_filter")
                .required(false)
                .action(ArgAction::Append)
                .long("env")
                .short('E')
                .value_name("KV")
                .value_parser(env_pass_validator)
                .help("Filter for this \"key=value\" environment variable")
            )
            .arg(Arg::new("image")
                .required(false)
                .long("image")
                .short('I')
                .value_name("IMAGE")
                .help("Only list artifacts that were built on IMAGE")
            )
        )

        .subcommand(Command::new("find-pkg")
            .about("Find a package by regex")
            .arg(Arg::new("package_name_regex")
                .required(true)
                .index(1)
                .value_name("REGEX")
                .help("The regex to match the package name against")
            )
            .arg(Arg::new("package_version_constraint")
                .required(false)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .help("A version constraint to search for (optional), e.g., '=1.0.0'")
            )

            .arg(Arg::new("terse")
                .action(ArgAction::SetTrue)
                .required(false)
                .long("terse")
                .short('t')
                .help("Do not use the fancy format, but simply <name> <version>")
            )

            .arg(Arg::new("show_all")
                .action(ArgAction::SetTrue)
                .required(false)
                .long("all")
                .short('A')
                .help("Same as: -SDpEFPs --denied-images --allowed-images (all flags enabled)")
            )

            .arg(Arg::new("show_sources")
                .action(ArgAction::SetTrue)
                .required(false)
                .long("source")
                .alias("sources")
                .short('S')
                .help("Show the sources of the package")
            )

            .arg(Arg::new("show_dependencies")
                .action(ArgAction::SetTrue)
                .required(false)
                .long("dependencies")
                .alias("deps")
                .short('D')
                .help("Show the dependencies of the package")
            )
            .arg(Arg::new("dependency_type")
                .required(false)
                .action(ArgAction::Append)
                .long("dependency-type")
                .value_name("DEPENDENCY_TYPE")
                .value_parser([
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .default_values([
                    IDENT_DEPENDENCY_TYPE_BUILD,
                    IDENT_DEPENDENCY_TYPE_RUNTIME,
                ])
                .help("Specify which dependency types are to print.")
            )

            .arg(Arg::new("show_patches")
                .action(ArgAction::SetTrue)
                .required(false)
                .long("patches")
                .short('p')
                .help("Show the patches of the package")
            )

            .arg(Arg::new("show_env")
                .action(ArgAction::SetTrue)
                .required(false)
                .long("env")
                .short('E')
                .help("Show the environment of the package")
            )

            .arg(Arg::new("show_flags")
                .action(ArgAction::SetTrue)
                .required(false)
                .long("flags")
                .short('F')
                .help("Show the flags of the package")
            )

            .arg(Arg::new("show_allowed_images")
                .action(ArgAction::SetTrue)
                .required(false)
                .long("allowed-images")
                .help("Show the images on which the package is only allowed to be built")
            )

            .arg(Arg::new("show_denied_images")
                .action(ArgAction::SetTrue)
                .required(false)
                .long("denied-images")
                .help("Show the images on which the package is not allowed to be built")
            )

            .arg(Arg::new("show_phases")
                .action(ArgAction::SetTrue)
                .required(false)
                .long("phases")
                .short('P')
                .help("Show the phases of the script of the package")
            )

            .arg(Arg::new("show_script")
                .action(ArgAction::SetTrue)
                .required(false)
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
            .about("Handle package sources")
            .subcommand(Command::new("verify")
                .about("Hash-check all source files")
                .arg(Arg::new("package_name")
                    .required(false)
                    .index(1)
                    .value_name("PKG")
                    .help("Verify the sources of this package (optional, if left out, all packages are checked)")
                )
                .arg(Arg::new("package_version")
                    .required(false)
                    .index(2)
                    .value_name("VERSION")
                    .help("Verify the sources of this package version (optional, if left out, all packages are checked)")
                )

                .arg(Arg::new("matching")
                    .required(false)
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
                .about("List packages where the source is missing")
            )
            .subcommand(Command::new("url")
                .about("Show the URL of the source of a package")
                .arg(Arg::new("package_name")
                    .required(false)
                    .index(1)
                    .value_name("PKG")
                    .help("Verify the sources of this package (optional, if left out, all packages are checked)")
                )
                .arg(Arg::new("package_version")
                    .required(false)
                    .index(2)
                    .value_name("VERSION")
                    .help("Verify the sources of this package version (optional, if left out, all packages are checked)")
                )
            )
            .subcommand(Command::new("download")
                .about("Download the source for one or multiple packages")
                .arg(Arg::new("package_name")
                    .required(false)
                    .index(1)
                    .value_name("PKG")
                    .help("Download the sources of this package")
                )
                .arg(Arg::new("package_version")
                    .required(false)
                    .index(2)
                    .value_name("VERSION_CONSTRAINT")
                    .help("Download the sources of this package version (optional, if left out, all packages are downloaded)")
                )
                .arg(Arg::new("force")
                    .action(ArgAction::SetTrue)
                    .required(false)
                    .long("force")
                    .help("Overwrite existing cache entry")
                )

                .arg(Arg::new("matching")
                    .required(false)
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
                    .long("timeout")
                    .value_name("TIMEOUT")
                    .help("Set timeout for download in seconds")
                    .value_parser(clap::value_parser!(u64))
                )

                .arg(Arg::new("recursive")
                    .action(ArgAction::SetTrue)
                    .required(false)
                    .long("recursive")
                    .help("Download the sources and all the dependency sources")
                )

                .arg(Arg::new("image")
                    .required(false)
                    .value_name("IMAGE NAME")
                    .short('I')
                    .long("image")
                    .help("Name of the Docker image to use")
                    .long_help(indoc::indoc!(r#"
                        Name of the Docker image to use.

                        Required because tree might look different on different images because of
                        conditions on dependencies.
                    "#))
                )

                .arg(Arg::new("env")
                    .required(false)
                    .action(ArgAction::Append)
                    .short('E')
                    .long("env")
                    .value_parser(env_pass_validator)
                    .help("Additional env to be passed when building packages")
                    .long_help(indoc::indoc!(r#"
                        Additional env to be passed when building packages.

                        Required because tree might look different on different images because of
                        conditions on dependencies.
                    "#))
                )
            )
            .subcommand(Command::new("of")
                .about("Get the paths of the sources of a package")
                .arg(Arg::new("package_name")
                    .required(false)
                    .index(1)
                    .value_name("PKG")
                    .help("Get the source file paths for this package")
                )
                .arg(Arg::new("package_version")
                    .required(false)
                    .index(2)
                    .value_name("VERSION")
                    .help("Get the source file paths for the package in this version")
                )
            )
        )

        .subcommand(Command::new("release")
            .about("Manage artifact releases")
            .subcommand(releases_list_command.name("list"))
            .subcommand(Command::new("rm")
                .about("Remove release artifacts")
                .long_about(indoc::indoc!(r#"
                    Removes a released artifact from the release store and deletes the according database entry.

                    This command asks interactively whether you want to delete data.
                    This can't be turned off.
                "#))
                .arg(Arg::new("release_store_name")
                    .required(true)
                    .long("from")
                    .value_name("RELEASE_STORE_NAME")
                    .help("Release store name to remove release from")
                )

                .arg(Arg::new("package_name")
                    .required(false)
                    .index(1)
                    .value_name("PKG")
                    .help("The name of the package")
                    .requires("package_version")
                )

                .arg(Arg::new("package_version")
                    .required(false)
                    .index(2)
                    .value_name("VERSION")
                    .help("The exact version of the package (string match)")
                    .requires("package_name")
                )
            )

            .subcommand(Command::new("new")
                .about("Release artifacts")
                .arg(Arg::new("submit_uuid")
                    .required(true)
                    .index(1)
                    .value_name("SUBMIT")
                    .help("The submit uuid from which to release a package")
                    .value_parser(uuid::Uuid::parse_str)
                )
                .arg(Arg::new("release_store_name")
                    .required(true)
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
                    .index(2)
                    .value_name("PKG")
                    .help("The name of the package")
                    .conflicts_with("all-packages")
                )
                .arg(Arg::new("all-packages")
                    .required(false)
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
                    .index(3)
                    .value_name("VERSION")
                    .help("The exact version of the package (string match)")
                )
                .arg(Arg::new("package_do_update")
                    .action(ArgAction::SetTrue)
                    .required(false)
                    .long("update")
                    .help("Do update a package if it already exists in the release store")
                )
                .arg(Arg::new("noninteractive")
                    .action(ArgAction::SetTrue)
                    .required(false)
                    .long("non-interactive")
                    .help("Don't be interactive (only with --update at the moment)")
                    .requires("package_do_update")
                )
                .arg(Arg::new("quiet")
                    .action(ArgAction::SetTrue)
                    .required(false)
                    .long("quiet")
                    .short('q')
                    .help("Don't print the paths to released files after releases are complete")
                )
            )

        )

        .subcommand(Command::new("lint")
            .about("Lint the package script of one or multiple packages")
            .arg(Arg::new("package_name")
                .required(false)
                .index(1)
                .value_name("NAME")
                .help("Package name to lint (if not present, every package will be linted")
            )
            .arg(Arg::new("package_version")
                .required(false)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .help("A version constraint to search for (optional), e.g., '=1.0.0'")
            )
        )

        .subcommand(Command::new("tree-of")
            .about("Print the dependency tree of one or multiple packages")
            .arg(Arg::new("package_name")
                .required(true)
                .index(1)
                .value_name("NAME")
                .help("Package name to lint (if not present, every package will be linted")
            )
            .arg(Arg::new("package_version")
                .required(false)
                .index(2)
                .value_name("VERSION_CONSTRAINT")
                .help("A version constraint to search for (optional), e.g., '=1.0.0'")
            )
            .arg(Arg::new("image")
                .required(false)
                .value_name("IMAGE NAME")
                .short('I')
                .long("image")
                .help("Name of the Docker image to use")
                .long_help(indoc::indoc!(r#"
                    Name of the Docker image to use.

                    Required because tree might look different on different images because of
                    conditions on dependencies.
                "#))
            )
            .arg(Arg::new("env")
                .required(false)
                .action(ArgAction::Append)
                .short('E')
                .long("env")
                .value_parser(env_pass_validator)
                .help("Additional env to be passed when building packages")
                .long_help(indoc::indoc!(r#"
                    Additional env to be passed when building packages.

                    Required because tree might look different on different images because of
                    conditions on dependencies.
                "#))
            )
            .arg(Arg::new("dot")
                .action(ArgAction::SetTrue)
                .required(false)
                .long("dot")
                .help("Output the dependency DAG in the Graphviz DOT format")
            )
        )

        .subcommand(Command::new("metrics")
            .about("Print metrics about butido")
        )

        .subcommand(Command::new("endpoint")
            .about("Endpoint maintenance commands")
            .arg(Arg::new("endpoint_name")
                .required(false)
                .index(1)
                .value_name("ENDPOINT_NAME")
                .help("Endpoint to talk to, or all if not given")
            )

            .subcommand(Command::new("ping")
                .about("Ping the endpoint(s)")
                .arg(Arg::new("ping_n")
                    .required(false)
                    .long("times")
                    .short('n')
                    .value_name("N")
                    .default_value("10")
                    .help("How often to ping")
                    .value_parser(clap::value_parser!(u64))
                )
                .arg(Arg::new("ping_sleep")
                    .required(false)
                    .long("sleep")
                    .value_name("N")
                    .default_value("1")
                    .help("How long to sleep between pings")
                    .value_parser(clap::value_parser!(u64))
                )
            )
            .subcommand(Command::new("stats")
                .about("Get stats for the endpoint(s)")
                .arg(Arg::new("csv")
                    .action(ArgAction::SetTrue)
                    .required(false)
                    .long("csv")
                    .help("Format output as CSV")
                )
            )
            .subcommand(Command::new("containers")
                .about("Work with the containers of the endpoint(s)")
                .subcommand(Command::new("prune")
                    .about("Remove exited containers")
                    .arg(arg_older_than_date("Prune only containers older than DATE"))
                    .arg(arg_newer_than_date("Prune only containers newer than DATE"))
                )
                .subcommand(Command::new("stop")
                    .about("Stop running containers")
                    .arg(arg_older_than_date("Stop only containers older than DATE"))
                    .arg(arg_newer_than_date("Stop only containers newer than DATE"))
                    .arg(Arg::new("timeout")
                        .required(false)
                        .long("timeout")
                        .short('t')
                        .value_name("TIMEOUT")
                        .help("Timeout in seconds")
                        .value_parser(clap::value_parser!(u64))
                    )
                )
                .subcommand(Command::new("list")
                    .about("List the containers and stats about them")
                    .arg(Arg::new("csv")
                        .action(ArgAction::SetTrue)
                        .required(false)
                        .long("csv")
                        .help("Format output as CSV")
                    )

                    .arg(Arg::new("list_stopped")
                        .action(ArgAction::SetTrue)
                        .required(false)
                        .long("list-stopped")
                        .help("List stopped containers too")
                    )

                    .arg(Arg::new("filter_image")
                        .required(false)
                        .short('I')
                        .long("image")
                        .value_name("IMAGE")
                        .help("List only containers of IMAGE")
                    )

                    .arg(arg_older_than_date("List only containers older than DATE"))
                    .arg(arg_newer_than_date("List only containers newer than DATE"))
                )
                .subcommand(Command::new("top")
                    .about("List the processes of all containers")
                    .arg(Arg::new("csv")
                        .action(ArgAction::SetTrue)
                        .required(false)
                        .long("csv")
                        .help("List top output as CSV")
                    )
                    .arg(Arg::new("limit")
                        .required(false)
                        .long("limit")
                        .value_name("LIMIT")
                        .help("Only list LIMIT processes for each container")
                        .value_parser(clap::value_parser!(usize))
                    )
                )
            )
            .subcommand(Command::new("container")
                .about("Work with a specific container")
                .arg(Arg::new("container_id")
                    .required(true)
                    .index(1)
                    .value_name("CONTAINER_ID")
                    .help("Work with container CONTAINER_ID")
                )
                .subcommand(Command::new("top")
                    .about("List the container processes")
                    .arg(Arg::new("csv")
                        .action(ArgAction::SetTrue)
                        .required(false)
                        .long("csv")
                        .help("List top output as CSV")
                    )
                )
                .subcommand(Command::new("kill")
                    .about("Kill the container")
                    .arg(Arg::new("signal")
                        .required(false)
                        .index(1)
                        .value_name("SIGNAL")
                        .help("Kill container with this signal")
                    )
                )
                .subcommand(Command::new("delete")
                    .about("Delete the container")
                )
                .subcommand(Command::new("start")
                    .about("Start the container")
                )
                .subcommand(Command::new("stop")
                    .about("Stop the container")
                    .arg(Arg::new("timeout")
                        .required(false)
                        .long("timeout")
                        .value_name("DURATION")
                        .help("Timeout in seconds")
                        .value_parser(clap::value_parser!(u64))
                    )
                )
                .subcommand(Command::new("exec")
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
                    .about("Display details about the container")
                    .long_about("Display details about the container. Do not assume the output format to be stable.")
                )
            )
            .subcommand(Command::new("images")
                .about("Query images on endpoint(s)")
                .subcommand(Command::new("list")
                    .about("List images on endpoint(s)")
                    .arg(Arg::new("csv")
                        .action(ArgAction::SetTrue)
                        .required(false)
                        .long("csv")
                        .help("List top output as CSV")
                    )
                )
                .subcommand(Command::new("verify-present")
                    .about("Verify that all configured images are present on endpoint(s)")
                    .arg(Arg::new("csv")
                        .action(ArgAction::SetTrue)
                        .required(false)
                        .long("csv")
                        .help("List top output as CSV")
                    )
                )
            )
        )
}

fn script_arg_line_numbers() -> clap::Arg {
    Arg::new("script_line_numbers")
        .action(ArgAction::SetTrue)
        .required(false)
        .long("line-numbers")
        .help("Print script with line numbers (default)")
        .conflicts_with("no_script_line_numbers")
}

fn script_arg_no_line_numbers() -> clap::Arg {
    Arg::new("no_script_line_numbers")
        .action(ArgAction::SetTrue)
        .required(false)
        .long("no-line-numbers")
        .help("Print script without line numbers")
        .conflicts_with("script_line_numbers")
}

fn script_arg_highlight() -> clap::Arg {
    Arg::new("script_highlight")
        .action(ArgAction::SetTrue)
        .required(false)
        .long("highlighting")
        .alias("highlight")
        .help("Print script with highlighting (default)")
        .conflicts_with("no_script_highlight")
}

fn script_arg_no_highlight() -> clap::Arg {
    Arg::new("no_script_highlight")
        .action(ArgAction::SetTrue)
        .required(false)
        .long("no-highlighting")
        .alias("no-highlight")
        .help("Print script without highlighting")
        .conflicts_with("script_highlight")
}

/// Naive check whether 's' is a 'key=value' pair or an existing environment variable
///
/// TODO: Clean up this spaghetti code
fn env_pass_validator(s: &str) -> Result<String, String> {
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
            error!("Error during validation: '{}' is not a key-value pair", s);
            Err(s)
        }
        Ok((k, v)) => {
            debug!("Env pass validation: '{}={}'", k, v);
            Ok(s.to_owned())
        }
    }
}

fn dir_exists_validator(s: &str) -> Result<String, String> {
    if PathBuf::from(&s).is_dir() {
        Ok(s.to_owned())
    } else {
        Err(format!("Directory does not exist: {s}"))
    }
}

fn arg_older_than_date(about: &str) -> Arg {
    Arg::new("older_than")
        .required(false)
        .long("older-than")
        .value_name("DATE")
        .help(about.to_owned())
        .long_help(
            r#"
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

        "#,
        )
        .value_parser(parse_date_from_string)
}

fn arg_newer_than_date(about: &str) -> Arg {
    Arg::new("newer_than")
        .required(false)
        .long("newer-than")
        .value_name("DATE")
        .help(about.to_owned())
        .long_help(
            r#"
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

        "#,
        )
        .value_parser(parse_date_from_string)
}

fn parse_date_from_string(s: &str) -> std::result::Result<String, String> {
    humantime::parse_duration(s)
        .map_err(|e| e.to_string())
        .map(|_| ())
        .or_else(|_| {
            humantime::parse_rfc3339_weak(s)
                .map_err(|e| e.to_string())
                .map(|_| ())
        })
        .or_else(|_| {
            let s = format!("{s} 00:00:00");
            humantime::parse_rfc3339_weak(&s)
                .map_err(|e| e.to_string())
                .map(|_| ())
        })
        .map(|_| s.to_owned())
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
