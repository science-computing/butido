use crate::package::Package;
use crate::package::PackageName;
use crate::package::StringEqual;
use crate::package::PackageVersionConstraint;

use filters::ops::bool::Bool;
use filters::filter::Filter;

/// Helper function to build a package filter based on some flags and the package version
pub fn build_package_filter_by_dependency_name(
    name: String,
    check_system_dep: bool,
    check_system_runtime_dep: bool,
    check_build_dep: bool,
    check_runtime_dep: bool)
    -> impl filters::filter::Filter<Package>
{
    let n = name.clone(); // clone, so we can move into closure
    let filter_system_dep = move |p: &Package| {
        trace!("Checking whether any system depenency of {:?} is '{}'", p, n);
        p.dependencies().system().iter().any(|sys_build_dep| sys_build_dep.str_equal(&n))
    };

    let n = name.clone(); // clone, so we can move into closure
    let filter_system_runtime_dep = move |p: &Package| {
        trace!("Checking whether any system runtime depenency of {:?} is '{}'", p, n);
        p.dependencies().system_runtime().iter().any(|sys_rt_dep| sys_rt_dep.str_equal(&n))
    };

    let n = name.clone(); // clone, so we can move into closure
    let filter_build_dep = move |p: &Package| {
        trace!("Checking whether any build depenency of {:?} is '{}'", p, n);
        p.dependencies().build().iter().any(|build_dep| build_dep.str_equal(&n))
    };

    let n = name.clone(); // clone, so we can move into closure
    let filter_rt_dep = move |p: &Package| {
        trace!("Checking whether any runtime depenency of {:?} is '{}'", p, n);
        p.dependencies().runtime().iter().any(|rt_dep| rt_dep.str_equal(&n))
    };

    (Bool::new(check_system_dep).and(filter_system_dep))
        .or(Bool::new(check_system_runtime_dep).and(filter_system_runtime_dep))
        .or(Bool::new(check_build_dep).and(filter_build_dep))
        .or(Bool::new(check_runtime_dep).and(filter_rt_dep))
}

pub fn build_package_filter_by_name_and_version(
    name: PackageName, 
    version_constraint: Option<PackageVersionConstraint>)
    -> impl filters::filter::Filter<Package>
{
    let name_filter    = move |p: &Package| {
        trace!("Checking {:?} -> name == {}", p, name);
        *p.name() == name
    };
    let version_filter = move |p: &Package| {
        trace!("Checking {:?} -> version matches {:?}", p, version_constraint);
        version_constraint.as_ref().map(|c| {
            c.matches(p.version())
                .unwrap() // TODO: This cannot fail because matches() returns always Ok(_). Still, FIXME
                .is_true()
        }).unwrap_or(true)
    };

    name_filter.and(version_filter)
}


#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use filters::filter::Filter;

    use crate::package::Dependency;
    use crate::package::Dependencies;
    use crate::package::tests::package;
    use crate::package::tests::pname;
    use crate::package::tests::pversion;
    use crate::repository::Repository;

    fn setup_logging() {
        let _ = env_logger::try_init();
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

        let f = build_package_filter_by_dependency_name(String::from("foo"), false, false, false, false);

        let found = repo.packages()
            .filter(|p| f.filter(p))
            .collect::<Vec<_>>();

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
            pack.set_dependencies(Dependencies::with_runtime_dependency(Dependency::from(String::from("foo"))));
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);

        let f = build_package_filter_by_dependency_name(String::from("foo"), false, false, false, false);

        let found = repo.packages()
            .filter(|p| f.filter(p))
            .collect::<Vec<_>>();

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
            pack.set_dependencies(Dependencies::with_runtime_dependency(Dependency::from(String::from("foo"))));
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);

        let f = build_package_filter_by_dependency_name(String::from("foo"), false, false, false, true);

        let found = repo.packages()
            .filter(|p| f.filter(p))
            .collect::<Vec<_>>();

        assert_eq!(found.len(), 1);
        let p = found.get(0).unwrap();
        assert_eq!(*p.name(), pname("a"));
        assert_eq!(*p.dependencies().runtime(), vec![Dependency::from(String::from("foo"))]);
        assert!(p.dependencies().build().is_empty());
        assert!(p.dependencies().system().is_empty());
        assert!(p.dependencies().system_runtime().is_empty());
    }

    #[test]
    fn test_filter_for_foo_with_bar_dep_disabled_filtering() {
        let mut btree = BTreeMap::new();

        {
            let name = "a";
            let vers = "1";
            let mut pack = package(name, vers, "https://rust-lang.org", "123");
            pack.set_dependencies(Dependencies::with_runtime_dependency(Dependency::from(String::from("bar"))));
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);

        let f = build_package_filter_by_dependency_name(String::from("foo"), false, false, false, false);

        let found = repo.packages()
            .filter(|p| f.filter(p))
            .collect::<Vec<_>>();

        assert_eq!(found.len(), 0);
    }

    #[test]
    fn test_filter_for_foo_with_bar_dep_enabled_filtering() {
        let mut btree = BTreeMap::new();

        {
            let name = "a";
            let vers = "1";
            let mut pack = package(name, vers, "https://rust-lang.org", "123");
            pack.set_dependencies(Dependencies::with_runtime_dependency(Dependency::from(String::from("bar"))));
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);

        let f = build_package_filter_by_dependency_name(String::from("foo"), false, false, false, true);

        let found = repo.packages()
            .filter(|p| f.filter(p))
            .collect::<Vec<_>>();

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
                                                        Dependency::from(String::from("foo")),
                                                        Dependency::from(String::from("bar"))
                                                        ])
            });
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);

        let f = build_package_filter_by_dependency_name(String::from("foo"), false, false, false, true);

        let found = repo.packages()
            .filter(|p| f.filter(p))
            .collect::<Vec<_>>();

        assert_eq!(found.len(), 1);
        let p = found.get(0).unwrap();
        assert_eq!(*p.name(), pname("a"));
        assert_eq!(*p.dependencies().runtime(), vec![Dependency::from(String::from("foo")), Dependency::from(String::from("bar"))]);
        assert!(p.dependencies().build().is_empty());
        assert!(p.dependencies().system().is_empty());
        assert!(p.dependencies().system_runtime().is_empty());
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
                                                        Dependency::from(String::from("foo")),
                                                        Dependency::from(String::from("bar"))
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
                                                        Dependency::from(String::from("foo")),
                                                        Dependency::from(String::from("baz"))
                                                        ])
            });
            btree.insert((pname(name), pversion(vers)), pack);
        }

        let repo = Repository::from(btree);

        let f = build_package_filter_by_dependency_name(String::from("foo"), false, false, false, true);

        let found = repo.packages()
            .filter(|p| f.filter(p))
            .collect::<Vec<_>>();

        assert_eq!(found.len(), 2);

        {
            let p = found.get(0).unwrap();
            assert_eq!(*p.name(), pname("a"));
            assert_eq!(*p.dependencies().runtime(), vec![Dependency::from(String::from("foo")), Dependency::from(String::from("bar"))]);
            assert!(p.dependencies().build().is_empty());
            assert!(p.dependencies().system().is_empty());
            assert!(p.dependencies().system_runtime().is_empty());
        }

        {
            let p = found.get(1).unwrap();
            assert_eq!(*p.name(), pname("b"));
            assert_eq!(*p.dependencies().runtime(), vec![Dependency::from(String::from("foo")), Dependency::from(String::from("baz"))]);
            assert!(p.dependencies().build().is_empty());
            assert!(p.dependencies().system().is_empty());
            assert!(p.dependencies().system_runtime().is_empty());
        }
    }

}
