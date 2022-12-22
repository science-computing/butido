//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::package::PackageName;
use crate::package::PackageVersionConstraint;
use crate::package::dependency::ParseDependency;
use crate::package::dependency::StringEqual;
use crate::package::dependency::condition::Condition;

/// A dependency that is packaged and is required during runtime
#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[serde(untagged)]
pub enum Dependency {
    Simple(String),
    Conditional {
        name: String,
        condition: Condition,
    },
}

#[cfg(test)]
impl Dependency {
    pub fn new_conditional(name: String, condition: Condition) -> Self {
        Dependency::Conditional { name, condition }
    }
}

impl AsRef<str> for Dependency {
    fn as_ref(&self) -> &str {
        match self {
            Dependency::Simple(name) => name,
            Dependency::Conditional { name, .. } => name,
        }
    }
}

impl StringEqual for Dependency {
    fn str_equal(&self, s: &str) -> bool {
        match self {
            Dependency::Simple(name) => name == s,
            Dependency::Conditional { name, .. } => name == s,
        }
    }
}

impl From<String> for Dependency {
    fn from(s: String) -> Dependency {
        Dependency::Simple(s)
    }
}

impl ParseDependency for Dependency {
    fn parse_as_name_and_version(&self) -> Result<(PackageName, PackageVersionConstraint)> {
        crate::package::dependency::parse_package_dependency_string_into_name_and_version(self.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::dependency::condition::OneOrMore;

    #[derive(serde::Deserialize)]
    #[allow(unused)]
    pub struct TestSetting {
        setting: Dependency,
    }

    #[test]
    fn test_parse_dependency() {
        let s: TestSetting = toml::from_str(r#"setting = "foo""#).expect("Parsing TestSetting failed");

        match s.setting {
            Dependency::Simple(name) => assert_eq!(name, "foo", "Expected 'foo', got {name}"),
            other => panic!("Unexpected deserialization to other variant: {:?}", other),
        }
    }

    #[test]
    fn test_parse_conditional_dependency() {
        let s: TestSetting = toml::from_str(r#"setting = { name = "foo", condition = { in_image = "bar"} }"#).expect("Parsing TestSetting failed");
        match s.setting {
            Dependency::Conditional { name, condition } => {
                assert_eq!(name, "foo", "Expected 'foo', got {name}");
                assert_eq!(*condition.has_env(), None);
                assert_eq!(*condition.env_eq(), None);
                assert_eq!(condition.in_image().as_ref(), Some(&OneOrMore::<String>::One(String::from("bar"))));
            },
            other => panic!("Unexpected deserialization to other variant: {:?}", other),
        }
    }

    #[test]
    fn test_parse_conditional_dependency_pretty() {
        let pretty = r#"
            [setting]
            name = "foo"
            [setting.condition]
            in_image = "bar"
        "#;

        let s: TestSetting = toml::from_str(pretty).expect("Parsing TestSetting failed");

        match s.setting {
            Dependency::Conditional { name, condition } => {
                assert_eq!(name, "foo", "Expected 'foo', got {name}");
                assert_eq!(*condition.has_env(), None);
                assert_eq!(*condition.env_eq(), None);
                assert_eq!(condition.in_image().as_ref(), Some(&OneOrMore::<String>::One(String::from("bar"))));
            },
            other => panic!("Unexpected deserialization to other variant: {:?}", other),
        }
    }


    #[derive(serde::Serialize, serde::Deserialize)]
    #[allow(unused)]
    pub struct TestSettings {
        settings: Vec<Dependency>,
    }

    #[test]
    fn test_parse_conditional_dependencies() {
        let s: TestSettings = toml::from_str(r#"settings = [{ name = "foo", condition = { in_image = "bar"} }]"#).expect("Parsing TestSetting failed");
        match s.settings.get(0).expect("Has not one dependency") {
            Dependency::Conditional { name, condition } => {
                assert_eq!(name, "foo", "Expected 'foo', got {name}");
                assert_eq!(*condition.has_env(), None);
                assert_eq!(*condition.env_eq(), None);
                assert_eq!(condition.in_image().as_ref(), Some(&OneOrMore::<String>::One(String::from("bar"))));
            },
            other => panic!("Unexpected deserialization to other variant: {:?}", other),
        }
    }

    #[test]
    fn test_parse_conditional_dependencies_pretty() {
        let pretty = r#"
            [[settings]]
            name = "foo"
            condition = { in_image = "bar" }
        "#;

        let s: TestSettings = toml::from_str(pretty).expect("Parsing TestSetting failed");

        match s.settings.get(0).expect("Has not one dependency") {
            Dependency::Conditional { name, condition } => {
                assert_eq!(name, "foo", "Expected 'foo', got {name}");
                assert_eq!(*condition.has_env(), None);
                assert_eq!(*condition.env_eq(), None);
                assert_eq!(condition.in_image().as_ref(), Some(&OneOrMore::<String>::One(String::from("bar"))));
            },
            other => panic!("Unexpected deserialization to other variant: {:?}", other),
        }
    }

    #[test]
    fn test_parse_conditional_dependencies_pretty_2() {
        let pretty = r#"
            [[settings]]
            name = "foo"
            condition.in_image = "bar"
        "#;

        let s: TestSettings = toml::from_str(pretty).expect("Parsing TestSetting failed");

        match s.settings.get(0).expect("Has not one dependency") {
            Dependency::Conditional { name, condition } => {
                assert_eq!(name, "foo", "Expected 'foo', got {name}");
                assert_eq!(*condition.has_env(), None);
                assert_eq!(*condition.env_eq(), None);
                assert_eq!(condition.in_image().as_ref(), Some(&OneOrMore::<String>::One(String::from("bar"))));
            },
            other => panic!("Unexpected deserialization to other variant: {:?}", other),
        }
    }

    #[test]
    fn test_parse_conditional_dependencies_pretty_3() {
        let pretty = r#"
            [[settings]]
            name = "foo"
            condition.in_image = "bar"

            [[settings]]
            name = "baz"
            condition.in_image = "boogie"
        "#;

        let s: TestSettings = toml::from_str(pretty).expect("Parsing TestSetting failed");

        match s.settings.get(0).expect("Has not one dependencies") {
            Dependency::Conditional { name, condition } => {
                assert_eq!(name, "foo", "Expected 'foo', got {name}");
                assert_eq!(*condition.has_env(), None);
                assert_eq!(*condition.env_eq(), None);
                assert_eq!(condition.in_image().as_ref(), Some(&OneOrMore::<String>::One(String::from("bar"))));
            },
            other => panic!("Unexpected deserialization to other variant: {:?}", other),
        }

        match s.settings.get(1).expect("Has not two dependencies") {
            Dependency::Conditional { name, condition } => {
                assert_eq!(name, "baz", "Expected 'baz', got {name}");
                assert_eq!(*condition.has_env(), None);
                assert_eq!(*condition.env_eq(), None);
                assert_eq!(condition.in_image().as_ref(), Some(&OneOrMore::<String>::One(String::from("boogie"))));
            },
            other => panic!("Unexpected deserialization to other variant: {:?}", other),
        }
    }
}

