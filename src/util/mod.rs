use serde::Deserialize;

#[derive(Deserialize, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct EnvironmentVariableName(String);

pub mod filters;
pub mod git;
pub mod parser;
pub mod progress;

pub mod docker {
    use anyhow::Result;
    use anyhow::anyhow;
    use serde::Serialize;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
    pub struct ImageName(String);

    impl From<String> for ImageName {
        fn from(s: String) -> Self {
            ImageName(s)
        }
    }

    impl AsRef<str> for ImageName {
        fn as_ref(&self) -> &str {
            self.0.as_ref()
        }
    }

    /// Check whether a string is a valid docker tag name
    ///
    /// From the docker spec:
    ///
    /// > A tag name must be valid ASCII and may contain lowercase and uppercase letters, digits,
    /// > underscores, periods and dashes. A tag name may not start with a period or a dash and may
    /// > contain a maximum of 128 characters.
    ///
    /// Returns Ok(()) if `s` is a valid docker tag name, otherwise an explanatory error message
    // TODO: Remove allow(unused)
    #[allow(unused)]
    pub fn is_valid_tag_name(s: &str) -> Result<()> {
        let valid_chars = s.chars().all(|c| {
            c == '_' ||
            c == ':' ||
            c == '-' ||
            c.is_ascii_alphanumeric()
        });

        if !valid_chars {
            return Err(anyhow!("Invalid characters"))
        }

        if s.chars().count() > 128 {
            return Err(anyhow!("Too long"))
        }


        if s.chars().next().map(|c| c == '.' || c == '-').unwrap_or(false) {
            return Err(anyhow!("Starts with invalid character"))
        }

        Ok(())
    }
}

#[cfg(test)]
mod docker_test {
    extern crate env_logger;
    fn setup_logging() {
        let _ = env_logger::try_init();
    }

    use super::docker::*;

    #[test]
    fn is_valid_tag_name_test_1() {
        setup_logging();
        let test = |s| {
            debug!("check if valid: '{}'", s);
            let e = is_valid_tag_name(s);
            debug!("Result = {:?}", e);
            e
        };

        assert!(test("foo").is_ok());
        assert!(test("foo:bar").is_ok());
        assert!(test("foo123").is_ok());
        assert!(test("1f23oo").is_ok());
        assert!(test(":foo").is_ok());
        assert!(test(".foo").is_err());
        assert!(test("-foo").is_err());
    }

}

