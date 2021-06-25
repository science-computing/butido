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
use serde::Serialize;
use getset::Getters;

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
#[derive(Serialize, Deserialize, Getters, Clone, Debug, Eq, PartialEq)]
pub struct Condition {
    #[serde(rename = "has_env", skip_serializing_if = "Option::is_none")]
    #[getset(get = "pub")]
    has_env: Option<OneOrMore<EnvironmentVariableName>>,

    #[serde(rename = "env_eq", skip_serializing_if = "Option::is_none")]
    #[getset(get = "pub")]
    env_eq: Option<HashMap<EnvironmentVariableName, String>>,

    #[serde(rename = "in_image", skip_serializing_if = "Option::is_none")]
    #[getset(get = "pub")]
    in_image: Option<OneOrMore<String>>,
}

/// Manual implementation of PartialOrd for Condition
///
/// Because HashMap does not implement PartialOrd
impl PartialOrd for Condition {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering as O;

        let cmp_has_env = match (self.has_env.as_ref(), other.has_env.as_ref()) {
            (Some(a), Some(b)) => a.partial_cmp(b),
            (Some(_), None)    => Some(O::Greater),
            (None, Some(_))    => Some(O::Less),
            (None, None)       => Some(O::Equal),
        };

        if cmp_has_env.as_ref().map(|o| *o != O::Equal).unwrap_or(false) {
            return cmp_has_env
        }

        let cmp_env_eq = match (self.env_eq.as_ref(), other.env_eq.as_ref()) {
            // TODO: Is this safe? We ignore the HashMaps here and just say they are equal. They are most certainly not.
            (Some(_), Some(_)) => Some(O::Equal),
            (Some(_), None)    => Some(O::Greater),
            (None, Some(_))    => Some(O::Less),
            (None, None)       => Some(O::Equal),
        };

        if cmp_env_eq.as_ref().map(|o| *o != O::Equal).unwrap_or(false) {
            return cmp_env_eq
        }

        match (self.in_image.as_ref(), other.in_image.as_ref()) {
            (Some(a), Some(b)) => a.partial_cmp(b),
            (Some(_), None)    => Some(O::Greater),
            (None, Some(_))    => Some(O::Less),
            (None, None)       => Some(O::Equal),
        }
    }
}

/// Manual implementation of Ord for Condition
///
/// Because HashMap does not implement Ord
impl Ord for Condition {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Manual implementation of Hash for Condition
///
/// Because HashMap does not implement Hash
impl std::hash::Hash for Condition {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.has_env.hash(state);
        if let Some(hm) = self.env_eq.as_ref() {
            hm.iter().for_each(|(k, v)| (k, v).hash(state));
        };
        self.in_image.hash(state);
    }
}


/// Helper type for supporting Vec<T> and T in value
/// position of Condition
#[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
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
