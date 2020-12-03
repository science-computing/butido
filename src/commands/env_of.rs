use anyhow::Result;
use clap::ArgMatches;

use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::repository::Repository;

pub async fn env_of(matches: &ArgMatches, repo: Repository) -> Result<()> {
    use filters::filter::Filter;
    use std::io::Write;

    let package_filter = {
        let name       = matches.value_of("package_name").map(String::from).map(PackageName::from).unwrap();
        let constraint = matches.value_of("package_version_constraint").map(String::from).map(PackageVersionConstraint::new).unwrap()?;
        trace!("Checking for package with name = {} and version = {:?}", name, constraint);

        crate::util::filters::build_package_filter_by_name(name)
            .and(crate::util::filters::build_package_filter_by_version_constraint(constraint))
    };

    let mut stdout = std::io::stdout();
    repo.packages()
        .filter(|package| package_filter.filter(package))
        .inspect(|pkg| trace!("Found package: {:?}", pkg))
        .map(|pkg| {
            if let Some(hm) = pkg.environment() {
                for (key, value) in hm {
                    writeln!(stdout, "{} = '{}'", key, value)?;
                }
            } else {
                writeln!(stdout, "No environment")?;
            }

            Ok(())
        })
        .collect::<Result<_>>()
}



