use anyhow::Result;
use clap::ArgMatches;
use log::trace;

use crate::commands::util::getbool;
use crate::config::*;
use crate::package::PackageName;
use crate::repository::Repository;

pub async fn dependencies_of(matches: &ArgMatches, config: &Configuration, repo: Repository) -> Result<()> {
    use filters::filter::Filter;

    let package_filter = {
        let name = matches.value_of("package_name").map(String::from).map(PackageName::from).unwrap();
        trace!("Checking for package with name = {}", name);

        crate::util::filters::build_package_filter_by_name(name)
    };

    let format = config.package_print_format();
    let mut stdout = std::io::stdout();
    let iter = repo.packages().filter(|package| package_filter.filter(package))
        .inspect(|pkg| trace!("Found package: {:?}", pkg));

    let print_runtime_deps     = getbool(matches, "dependency_type", crate::cli::IDENT_DEPENDENCY_TYPE_RUNTIME);
    let print_build_deps       = getbool(matches, "dependency_type", crate::cli::IDENT_DEPENDENCY_TYPE_BUILD);

    trace!("Printing packages with format = '{}', runtime: {}, build: {}",
           format,
           print_runtime_deps,
           print_build_deps);

    crate::ui::print_packages(&mut stdout,
                       format,
                       iter,
                       config,
                       print_runtime_deps,
                       print_build_deps,
                       false, // "print_sources"
                       true,  // "print_dependencies"
                       false, // "print_patches"
                       false, // "print_env"
                       false, // "print_flags"
                       false, // "print_deny_images"
                       false, // "print_phases"
                       false, // "print_script"
                       )
}

