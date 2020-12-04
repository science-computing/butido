use anyhow::Result;
use clap::ArgMatches;
use log::trace;
use resiter::Filter;
use resiter::Map;

use crate::commands::util::getbool;
use crate::config::*;
use crate::package::PackageName;
use crate::repository::Repository;

pub async fn what_depends(matches: &ArgMatches, config: &Configuration, repo: Repository) -> Result<()> {
    use filters::failable::filter::FailableFilter;

    let print_runtime_deps     = getbool(matches, "dependency_type", crate::cli::IDENT_DEPENDENCY_TYPE_RUNTIME);
    let print_build_deps       = getbool(matches, "dependency_type", crate::cli::IDENT_DEPENDENCY_TYPE_BUILD);

    let package_filter = {
        let name = matches.value_of("package_name").map(String::from).map(PackageName::from).unwrap();

        crate::util::filters::build_package_filter_by_dependency_name(
            &name,
            print_build_deps,
            print_runtime_deps
        )
    };

    let format = config.package_print_format();
    let mut stdout = std::io::stdout();

    let packages = repo.packages()
        .map(|package| package_filter.filter(package).map(|b| (b, package)))
        .filter_ok(|(b, _)| *b)
        .map_ok(|tpl| tpl.1)
        .inspect(|pkg| trace!("Found package: {:?}", pkg))
        .collect::<Result<Vec<_>>>()?;

    crate::ui::print_packages(&mut stdout,
                       format,
                       packages.into_iter(),
                       print_runtime_deps,
                       print_build_deps)
}

