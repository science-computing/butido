use anyhow::Error;
use anyhow::Result;
use clap::ArgMatches;
use log::trace;

use crate::config::Configuration;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;

pub async fn find_pkg(matches: &ArgMatches, config: &Configuration, repo: Repository) -> Result<()> {
    use std::io::Write;

    let package_name_regex = matches
        .value_of("package_name_regex")
        .map(regex::RegexBuilder::new)
        .map(|mut builder| {
            builder.size_limit(1 * 1024 * 1024); // max size for the regex is 1MB. Should be enough for everyone
            builder.build()
                .map_err(Error::from)
        })
        .unwrap()?; // safe by clap

    let package_version_constraint = matches
        .value_of("package_version_constraint")
        .map(String::from)
        .map(PackageVersionConstraint::new)
        .transpose()?;

    let iter = repo.packages()
        .filter(|p| package_name_regex.captures(p.name()).is_some())
        .filter(|p| package_version_constraint.as_ref().map(|v| v.matches(p.version())).unwrap_or(true))
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
            true)
    }
}



