//
// Copyright (c) 2020-2021 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::collections::HashMap;

use serde::Deserialize;

use crate::util::EnvironmentVariableName;

/// The Condition type
///
/// This type represents a condition whether a dependency should be included in the package tree or
/// not.
///
/// Right now, we are supporting condition by environment (set or equal) or whether a specific
/// build image is used.
/// All these settings are optional, of course.
///
#[derive(Deserialize, Clone, Debug)]
pub struct Condition {
    #[serde(rename = "has_env", skip_serializing_if = "Option::is_none")]
    has_env: Option<OneOrMore<EnvironmentVariableName>>,

    #[serde(rename = "env_eq", skip_serializing_if = "Option::is_none")]
    env_eq: Option<HashMap<EnvironmentVariableName, String>>,

    #[serde(rename = "in_image", skip_serializing_if = "Option::is_none")]
    in_image: Option<OneOrMore<String>>,
}


/// Helper type for supporting Vec<T> and T in value
/// position of Condition
#[cfg_attr(test, derive(Eq, PartialEq))]
#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum OneOrMore<T: Sized> {
    One(T),
    More(Vec<T>),
}

impl<T: Sized> Into<Vec<T>> for OneOrMore<T> {
    fn into(self) -> Vec<T> {
        match self {
            OneOrMore::One(o) => vec![o],
            OneOrMore::More(m) => m,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_env_deserialization() {
        let s = r#"has_env = "foo""#;
        let c: Condition = toml::from_str(s).expect("Deserializing has_env");

        assert_eq!(c.has_env.unwrap(), OneOrMore::<EnvironmentVariableName>::One(EnvironmentVariableName::from("foo")));
        assert!(c.env_eq.is_none());
        assert!(c.in_image.is_none());
    }

    #[test]
    fn test_has_env_list_deserialization() {
        let s = r#"has_env = ["foo", "bar"]"#;
        let c: Condition = toml::from_str(s).expect("Deserializing has_env");

        assert_eq!(c.has_env.unwrap(), {
            OneOrMore::<EnvironmentVariableName>::More({
                vec![EnvironmentVariableName::from("foo"), EnvironmentVariableName::from("bar")]
            })
        });
        assert!(c.env_eq.is_none());
        assert!(c.in_image.is_none());
    }

    #[test]
    fn test_env_eq_deserialization() {
        let s = r#"env_eq = { "foo" = "bar" }"#;
        let c: Condition = toml::from_str(s).expect("Deserializing has_env");

        assert!(c.has_env.is_none());
        assert_eq!(c.env_eq.unwrap(), {
            let mut hm = HashMap::new();
            hm.insert(EnvironmentVariableName::from("foo"), String::from("bar"));
            hm
        });
        assert!(c.in_image.is_none());
    }

    #[test]
    fn test_in_image_deserialization() {
        let s = r#"in_image = "foo""#;
        let c: Condition = toml::from_str(s).expect("Deserializing has_env");

        assert!(c.has_env.is_none());
        assert!(c.env_eq.is_none());
        assert_eq!(c.in_image.unwrap(), OneOrMore::<String>::One(String::from("foo")));
    }

    #[test]
    fn test_in_image_list_deserialization() {
        let s = r#"in_image = ["foo"]"#;
        let c: Condition = toml::from_str(s).expect("Deserializing has_env");

        assert!(c.has_env.is_none());
        assert!(c.env_eq.is_none());
        assert_eq!(c.in_image.unwrap(), OneOrMore::<String>::More(vec![String::from("foo")]));
    }

}
