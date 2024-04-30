//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::Error;
use anyhow::Result;
use filters::failable::filter::FailableFilter;
use resiter::Map;
use tracing::trace;

use crate::package::Package;
use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::package::ParseDependency;

/// Helper function to build a package filter based on some flags and the package version
pub fn build_package_filter_by_dependency_name(
    name: &PackageName,
    check_build_dep: bool,
    check_runtime_dep: bool,
) -> impl filters::failable::filter::FailableFilter<Package, Error = Error> {
    let n = name.clone(); // clone, so we can move into closure
    let filter_build_dep = move |p: &Package| -> Result<bool> {
        trace!(
            "Checking whether any build dependency of {:?} is '{}'",
            p,
            n
        );
        Ok({
            check_build_dep
                && p.dependencies()
                    .build()
                    .iter()
                    .inspect(|d| trace!("Checking {:?}", d))
                    .map(|d| d.parse_as_name_and_version())
                    .map_ok(|(name, _)| name == n)
                    .collect::<Result<Vec<bool>>>()?
                    .into_iter()
                    .inspect(|b| trace!("found: {}", b))
                    .any(|b| b)
        })
    };

    let n = name.clone(); // clone, so we can move into closure
    let filter_rt_dep = move |p: &Package| -> Result<bool> {
        trace!(
            "Checking whether any runtime dependency of {:?} is '{}'",
            p,
            n
        );
        Ok({
            check_runtime_dep
                && p.dependencies()
                    .runtime()
                    .iter()
                    .inspect(|d| trace!("Checking {:?}", d))
                    .map(|d| d.parse_as_name_and_version())
                    .map_ok(|(name, _)| name == n)
                    .collect::<Result<Vec<bool>>>()?
                    .into_iter()
                    .inspect(|b| trace!("found: {}", b))
                    .any(|b| b)
        })
    };

    filter_build_dep.or(filter_rt_dep)
}

pub fn build_package_filter_by_name(name: PackageName) -> impl filters::filter::Filter<Package> {
    move |p: &Package| {
        trace!("Checking {:?} -> name == {}", p, name);
        *p.name() == name
    }
}

pub fn build_package_filter_by_version_constraint(
    constraint: PackageVersionConstraint,
) -> impl filters::filter::Filter<Package> {
    move |p: &Package| {
        trace!("Checking {:?} -> version matches {:?}", p, constraint);
        constraint.matches(p.version())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use resiter::Filter;

    use crate::package::tests::package;
    use crate::package::tests::pname;
    use crate::package::tests::pversion;
    use crate::package::Dependencies;
    use crate::package::Dependency;
    use crate::repository::Repository;

    fn setup_logging() {
        let _ = tracing_subscriber::fmt::try_init();
    }

    #[test]
    fn test_filter_for_foo_with_empty_deps() {
        setup_logging();

        let mut btree = BTreeMap::new();

        {
            let name = "a";
            let vers = "1";
            let mut pack = package(name, vers, "https://rust-lang.org", "123");
            pack.set_dependencies(Dependencies::empty());
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);

        let f = build_package_filter_by_dependency_name(&pname("foo"), false, false);

        let found = repo
            .packages()
            .map(|p| f.filter(p).map(|b| (b, p)))
            .filter_ok(|(b, _)| *b)
            .map_ok(|tpl| tpl.1)
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert!(found.is_empty());
    }

    #[test]
    fn test_filter_for_foo_with_foo_dep_but_disabled_filtering() {
        setup_logging();
        let mut btree = BTreeMap::new();

        {
            let name = "a";
            let vers = "1";
            let mut pack = package(name, vers, "https://rust-lang.org", "123");
            pack.set_dependencies(Dependencies::with_runtime_dependency(Dependency::from(
                String::from("foo"),
            )));
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);

        let f = build_package_filter_by_dependency_name(&pname("foo"), false, false);

        let found = repo
            .packages()
            .map(|p| f.filter(p).map(|b| (b, p)))
            .filter_ok(|(b, _)| *b)
            .map_ok(|tpl| tpl.1)
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn test_filter_for_foo_with_foo_dep_enabled_filtering() {
        setup_logging();
        let mut btree = BTreeMap::new();

        {
            let name = "a";
            let vers = "1";
            let mut pack = package(name, vers, "https://rust-lang.org", "123");
            pack.set_dependencies(Dependencies::with_runtime_dependency(Dependency::from(
                String::from("foo =2.0"),
            )));
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);

        let f = build_package_filter_by_dependency_name(&pname("foo"), false, true);

        let found = repo
            .packages()
            .map(|p| f.filter(p).map(|b| (b, p)))
            .filter_ok(|(b, _)| *b)
            .map_ok(|tpl| tpl.1)
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(found.len(), 1);
        let p = found.first().unwrap();
        assert_eq!(*p.name(), pname("a"));
        assert_eq!(
            *p.dependencies().runtime(),
            vec![Dependency::from(String::from("foo =2.0"))]
        );
        assert!(p.dependencies().build().is_empty());
    }

    #[test]
    fn test_filter_for_foo_with_bar_dep_disabled_filtering() {
        let mut btree = BTreeMap::new();

        {
            let name = "a";
            let vers = "1";
            let mut pack = package(name, vers, "https://rust-lang.org", "123");
            pack.set_dependencies(Dependencies::with_runtime_dependency(Dependency::from(
                String::from("bar =1337"),
            )));
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);

        let f = build_package_filter_by_dependency_name(&pname("foo"), false, false);

        let found = repo
            .packages()
            .map(|p| f.filter(p).map(|b| (b, p)))
            .filter_ok(|(b, _)| *b)
            .map_ok(|tpl| tpl.1)
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn test_filter_for_foo_with_bar_dep_enabled_filtering() {
        let mut btree = BTreeMap::new();

        {
            let name = "a";
            let vers = "1";
            let mut pack = package(name, vers, "https://rust-lang.org", "123");
            pack.set_dependencies(Dependencies::with_runtime_dependency(Dependency::from(
                String::from("bar =1.0"),
            )));
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);

        let f = build_package_filter_by_dependency_name(&pname("foo"), false, true);

        let found = repo
            .packages()
            .map(|p| f.filter(p).map(|b| (b, p)))
            .filter_ok(|(b, _)| *b)
            .map_ok(|tpl| tpl.1)
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn test_filter_for_foo_with_foo_and_bar_dep() {
        setup_logging();
        let mut btree = BTreeMap::new();

        {
            let name = "a";
            let vers = "1";
            let mut pack = package(name, vers, "https://rust-lang.org", "123");
            pack.set_dependencies({
                Dependencies::with_runtime_dependencies(vec![
                    Dependency::from(String::from("foo =1")),
                    Dependency::from(String::from("bar =1")),
                ])
            });
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);

        let f = build_package_filter_by_dependency_name(&pname("foo"), false, true);

        let found = repo
            .packages()
            .map(|p| f.filter(p).map(|b| (b, p)))
            .filter_ok(|(b, _)| *b)
            .map_ok(|tpl| tpl.1)
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(found.len(), 1);
        let p = found.first().unwrap();
        assert_eq!(*p.name(), pname("a"));
        assert_eq!(
            *p.dependencies().runtime(),
            vec![
                Dependency::from(String::from("foo =1")),
                Dependency::from(String::from("bar =1"))
            ]
        );
        assert!(p.dependencies().build().is_empty());
    }

    #[test]
    fn test_filter_two_packages() {
        let mut btree = BTreeMap::new();

        {
            let name = "a";
            let vers = "1";
            let mut pack = package(name, vers, "https://rust-lang.org", "123");
            pack.set_dependencies({
                Dependencies::with_runtime_dependencies(vec![
                    Dependency::from(String::from("foo =2")),
                    Dependency::from(String::from("bar =3")),
                ])
            });
            btree.insert((pname(name), pversion(vers)), pack);
        }

        {
            let name = "b";
            let vers = "2";
            let mut pack = package(name, vers, "https://rust-lang.org", "123");
            pack.set_dependencies({
                Dependencies::with_runtime_dependencies(vec![
                    Dependency::from(String::from("foo =4")),
                    Dependency::from(String::from("baz =5")),
                ])
            });
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);

        let f = build_package_filter_by_dependency_name(&pname("foo"), false, true);

        let found = repo
            .packages()
            .map(|p| f.filter(p).map(|b| (b, p)))
            .filter_ok(|(b, _)| *b)
            .map_ok(|tpl| tpl.1)
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert_eq!(found.len(), 2);

        {
            let p = found.first().unwrap();
            assert_eq!(*p.name(), pname("a"));
            assert_eq!(
                *p.dependencies().runtime(),
                vec![
                    Dependency::from(String::from("foo =2")),
                    Dependency::from(String::from("bar =3"))
                ]
            );
            assert!(p.dependencies().build().is_empty());
        }

        {
            let p = found.get(1).unwrap();
            assert_eq!(*p.name(), pname("b"));
            assert_eq!(
                *p.dependencies().runtime(),
                vec![
                    Dependency::from(String::from("foo =4")),
                    Dependency::from(String::from("baz =5"))
                ]
            );
            assert!(p.dependencies().build().is_empty());
        }
    }
}
