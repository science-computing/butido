//
// Copyright (c) 2020-2022 science+computing ag and other contributors
//
// This program and the accompanying materials are made
// available under the terms of the Eclipse Public License 2.0
// which is available at https://www.eclipse.org/legal/epl-2.0/
//
// SPDX-License-Identifier: EPL-2.0
//

use std::collections::BTreeMap;

use anyhow::Result;
use getset::Getters;
use serde::Deserialize;
use serde::Serialize;

use crate::util::docker::ImageName;
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
#[derive(Serialize, Deserialize, Getters, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Condition {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getset(get = "pub")]
    pub(super) has_env: Option<OneOrMore<EnvironmentVariableName>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[getset(get = "pub")]
    pub(super) env_eq: Option<BTreeMap<EnvironmentVariableName, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[getset(get = "pub")]
    pub(super) in_image: Option<OneOrMore<String>>,
}

impl Condition {
    #[cfg(test)]
    pub fn new(
        has_env: Option<OneOrMore<EnvironmentVariableName>>,
        env_eq: Option<BTreeMap<EnvironmentVariableName, String>>,
        in_image: Option<OneOrMore<String>>,
    ) -> Self {
        Condition {
            has_env,
            env_eq,
            in_image,
        }
    }

    /// Check whether the condition matches a certain set of data
    ///
    /// # Return value
    ///
    /// Always returns Ok(_) in the current implementation
    pub fn matches(&self, data: &ConditionData<'_>) -> Result<bool> {
        if !self.matches_env_cond(data)? {
            return Ok(false);
        }

        if !self.matches_env_eq_cond(data)? {
            return Ok(false);
        }

        if !self.matches_in_image_cond(data)? {
            return Ok(false);
        }

        Ok(true)
    }

    fn matches_env_cond(&self, data: &ConditionData<'_>) -> Result<bool> {
        if let Some(has_env_cond) = self.has_env.as_ref() {
            let b = match has_env_cond {
                OneOrMore::One(env) => data.env.iter().any(|(name, _)| env == name),
                OneOrMore::More(envs) => envs
                    .iter()
                    .all(|required_env| data.env.iter().any(|(name, _)| name == required_env)),
            };

            if !b {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn matches_env_eq_cond(&self, data: &ConditionData<'_>) -> Result<bool> {
        if let Some(env_eq_cond) = self.env_eq.as_ref() {
            let b = env_eq_cond.iter().all(|(req_env_name, req_env_val)| {
                data.env
                    .iter()
                    .find(|(env_name, _)| env_name == req_env_name)
                    .map(|(_, env_val)| env_val == req_env_val)
                    .unwrap_or(false)
            });

            if !b {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn matches_in_image_cond(&self, data: &ConditionData<'_>) -> Result<bool> {
        if let Some(in_image_cond) = self.in_image.as_ref() {
            let b = match in_image_cond {
                OneOrMore::One(req_image) => {
                    // because the image_name in the ConditionData is Option,
                    // which is a design-decision because the image can be not-specified (in the
                    // "tree-of" subcommand),
                    // we automatically use `false` as value here.
                    //
                    // That is because if we need to have a certain image (which is what this
                    // condition expresses), and there is no image specified in the ConditionData,
                    // we are by definition are NOT in this image.
                    data.image_name
                        .as_ref()
                        .map(|i| i.as_ref() == req_image)
                        .unwrap_or(false)
                }
                OneOrMore::More(req_images) => req_images.iter().any(|ri| {
                    data.image_name
                        .as_ref()
                        .map(|inam| inam.as_ref() == ri)
                        .unwrap_or(false)
                }),
            };

            Ok(b)
        } else {
            Ok(true)
        }
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

#[allow(clippy::from_over_into)]
impl<T: Sized> Into<Vec<T>> for OneOrMore<T> {
    fn into(self) -> Vec<T> {
        match self {
            OneOrMore::One(o) => vec![o],
            OneOrMore::More(m) => m,
        }
    }
}

#[cfg(test)]
impl From<Vec<String>> for OneOrMore<String> {
    fn from(v: Vec<String>) -> Self {
        OneOrMore::More(v)
    }
}

#[cfg(test)]
impl From<String> for OneOrMore<String> {
    fn from(s: String) -> Self {
        OneOrMore::One(s)
    }
}

#[derive(Debug)]
pub struct ConditionData<'a> {
    pub(crate) image_name: Option<&'a ImageName>,
    pub(crate) env: &'a [(EnvironmentVariableName, String)],
}

/// Trait for all things that have a condition that can be checked against ConditionData.
///
/// To be implemented by dependency types.
///
/// # Return value
///
/// Ok(true) if the dependency is relevant, considering the ConditionData
/// Ok(false) if the dependency should be ignored, considering the ConditionData
/// Err(_) if the condition checking failed (see `Condition::matches`)
///
pub trait ConditionCheckable {
    fn check_condition(&self, data: &ConditionData<'_>) -> Result<bool>;
}

impl ConditionCheckable for crate::package::BuildDependency {
    fn check_condition(&self, data: &ConditionData<'_>) -> Result<bool> {
        match self {
            // If the dependency is a simple one, e.g. "foo =1.2.3", there is no condition, so the
            // dependency has always to be used
            crate::package::BuildDependency::Simple(_) => Ok(true),
            crate::package::BuildDependency::Conditional { condition, .. } => {
                condition.matches(data)
            }
        }
    }
}

impl ConditionCheckable for crate::package::Dependency {
    fn check_condition(&self, data: &ConditionData<'_>) -> Result<bool> {
        match self {
            // If the dependency is a simple one, e.g. "foo =1.2.3", there is no condition, so the
            // dependency has always to be used
            crate::package::Dependency::Simple(_) => Ok(true),
            crate::package::Dependency::Conditional { condition, .. } => condition.matches(data),
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

        assert_eq!(
            c.has_env.unwrap(),
            OneOrMore::<EnvironmentVariableName>::One(EnvironmentVariableName::from("foo"))
        );
        assert!(c.env_eq.is_none());
        assert!(c.in_image.is_none());
    }

    #[test]
    fn test_has_env_list_deserialization() {
        let s = r#"has_env = ["foo", "bar"]"#;
        let c: Condition = toml::from_str(s).expect("Deserializing has_env");

        assert_eq!(c.has_env.unwrap(), {
            OneOrMore::<EnvironmentVariableName>::More({
                vec![
                    EnvironmentVariableName::from("foo"),
                    EnvironmentVariableName::from("bar"),
                ]
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
            let mut hm = BTreeMap::new();
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
        assert_eq!(
            c.in_image.unwrap(),
            OneOrMore::<String>::One(String::from("foo"))
        );
    }

    #[test]
    fn test_in_image_list_deserialization() {
        let s = r#"in_image = ["foo"]"#;
        let c: Condition = toml::from_str(s).expect("Deserializing has_env");

        assert!(c.has_env.is_none());
        assert!(c.env_eq.is_none());
        assert_eq!(
            c.in_image.unwrap(),
            OneOrMore::<String>::More(vec![String::from("foo")])
        );
    }

    #[test]
    fn test_condition_empty() {
        let data = ConditionData {
            image_name: None,
            env: &[],
        };

        let condition = Condition::new(None, None, None);

        assert!(condition.matches(&data).unwrap());
    }

    #[test]
    fn test_condition_no_image() {
        let data = ConditionData {
            image_name: None,
            env: &[],
        };

        let condition = Condition::new(None, None, {
            Some(OneOrMore::<String>::One(String::from("req_image")))
        });

        assert!(!condition.matches(&data).unwrap());
    }

    #[test]
    fn test_condition_matching_image() {
        let img = ImageName::from("required_image");
        let data = ConditionData {
            image_name: Some(&img),
            env: &[],
        };

        let condition = Condition::new(None, None, {
            Some(OneOrMore::<String>::One(String::from("required_image")))
        });

        assert!(condition.matches(&data).unwrap());
    }

    #[test]
    fn test_condition_nonmatching_image() {
        let img = ImageName::from("required_image");
        let data = ConditionData {
            image_name: Some(&img),
            env: &[],
        };

        let condition = Condition::new(None, None, {
            Some(OneOrMore::<String>::One(String::from("other_image")))
        });

        assert!(!condition.matches(&data).unwrap());
    }

    #[test]
    fn test_condition_required_env_missing() {
        let data = ConditionData {
            image_name: None,
            env: &[],
        };

        let condition = Condition::new(
            {
                Some(OneOrMore::<EnvironmentVariableName>::One(
                    EnvironmentVariableName::from("A"),
                ))
            },
            None,
            None,
        );

        assert!(!condition.matches(&data).unwrap());
    }

    #[test]
    fn test_condition_required_env_present() {
        let data = ConditionData {
            image_name: None,
            env: &[(EnvironmentVariableName::from("A"), String::from("1"))],
        };

        let condition = Condition::new(
            {
                Some(OneOrMore::<EnvironmentVariableName>::One(
                    EnvironmentVariableName::from("A"),
                ))
            },
            None,
            None,
        );

        assert!(condition.matches(&data).unwrap());
    }

    #[test]
    fn test_condition_required_env_values_missing() {
        let data = ConditionData {
            image_name: None,
            env: &[],
        };

        let condition = Condition::new(
            None,
            {
                let mut hm = BTreeMap::new();
                hm.insert(EnvironmentVariableName::from("A"), String::from("1"));
                Some(hm)
            },
            None,
        );

        assert!(!condition.matches(&data).unwrap());
    }

    #[test]
    fn test_condition_required_env_values_present_but_different() {
        let data = ConditionData {
            image_name: None,
            env: &[(EnvironmentVariableName::from("A"), String::from("1"))],
        };

        let condition = Condition::new(
            None,
            {
                let mut hm = BTreeMap::new();
                hm.insert(EnvironmentVariableName::from("A"), String::from("2"));
                Some(hm)
            },
            None,
        );

        assert!(!condition.matches(&data).unwrap());
    }

    #[test]
    fn test_condition_required_env_values_present_and_equal() {
        let data = ConditionData {
            image_name: None,
            env: &[(EnvironmentVariableName::from("A"), String::from("1"))],
        };

        let condition = Condition::new(
            None,
            {
                let mut hm = BTreeMap::new();
                hm.insert(EnvironmentVariableName::from("A"), String::from("1"));
                Some(hm)
            },
            None,
        );

        assert!(condition.matches(&data).unwrap());
    }
}
