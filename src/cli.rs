use clap_v3 as clap;
use clap::App;
use clap::Arg;
use clap::crate_authors;
use clap::crate_version;

pub fn cli<'a>() -> App<'a> {
    App::new("butido")
        .author(crate_authors!())
        .version(crate_version!())
        .about("Generic Build Orchestration System for building linux packages with docker")
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
}

