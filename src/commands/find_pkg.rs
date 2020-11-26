use anyhow::Error;
use anyhow::Context;
use anyhow::Result;
use clap::ArgMatches;

use crate::config::Configuration;
use crate::package::Package;
use crate::repository::Repository;

pub async fn find_pkg<'a>(matches: &ArgMatches, config: &Configuration<'a>, repo: Repository) -> Result<()> {
    use filters::filter::Filter;
    use std::io::Write;

    let package_filter = {
        let regex = matches
            .value_of("package_name_regex")
            .map(regex::RegexBuilder::new)
            .map(|mut builder| {
                builder.size_limit(1 * 1024 * 1024); // max size for the regex is 1MB. Should be enough for everyone
                builder.build()
                    .map_err(Error::from)
            })
            .unwrap()?; // safe by clap

        move |p: &Package| -> bool {
            regex.captures(p.name()).is_some()
        }
    };

    let iter = repo.packages()
        .filter(|package| package_filter.filter(package))
        .inspect(|pkg| trace!("Found package: {:?}", pkg));

    let out = std::io::stdout();
    let mut outlock = out.lock();
    if matches.is_present("terse") {
        for p in iter {
            writeln!(outlock, "{} {}", p.name(), p.version())?;
        }
        Ok(())
    } else {
        let format = config.package_print_format();
        crate::ui::print_packages(&mut outlock,
            format,
            iter,
            true,
            true,
            true,
            true)
    }
}



