use clap_v3 as clap;
use clap::App;
use clap::Arg;
use clap::crate_authors;
use clap::crate_version;

pub fn cli<'a>() -> App<'a> {
    App::new("db")
        .author(crate_authors!())
        .version(crate_version!())
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
        )

}

