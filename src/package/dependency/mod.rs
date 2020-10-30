mod system;
pub use system::*;

mod system_runtime;
pub use system_runtime::*;

mod build;
pub use build::*;

mod runtime;
pub use runtime::*;

pub trait StringEqual {
    fn str_equal(&self, s: &str) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::convert::TryInto;

    use crate::package::Package;
    use crate::package::PackageName;
    use crate::package::PackageVersion;
    use crate::package::PackageVersionConstraint;

    //
    // helper functions
    //

    fn name(s: &'static str) -> PackageName {
        PackageName::from(String::from(s))
    }

    fn exact(s: &'static str) -> PackageVersionConstraint {
        PackageVersionConstraint::Exact(PackageVersion::from(String::from(s)))
    }

    fn higher_as(s: &'static str) -> PackageVersionConstraint {
        PackageVersionConstraint::HigherAs(PackageVersion::from(String::from(s)))
    }

    //
    // tests
    //

    #[test]
    fn test_dependency_conversion_1() {
        let s = "vim =8.2";
        let d = Dependency::from(String::from(s));

        let (n, c) = d.try_into().unwrap();

        assert_eq!(n, name("vim"));
        assert_eq!(c, exact("8.2"));
    }

    #[test]
    fn test_dependency_conversion_2() {
        let s = "gtk15 >1b";
        let d = Dependency::from(String::from(s));

        let (n, c) = d.try_into().unwrap();

        assert_eq!(n, name("gtk15"));
        assert_eq!(c, higher_as("1b"));
    }
}
