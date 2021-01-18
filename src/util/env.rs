use anyhow::anyhow;
use anyhow::Result;

use crate::util::EnvironmentVariableName;

pub fn parse_to_env(s: &str) -> Result<(EnvironmentVariableName, String)> {
    let v = s.split('=').collect::<Vec<_>>();
    Ok((
        EnvironmentVariableName::from(
            *v.get(0)
                .ok_or_else(|| anyhow!("Environment variable has no key: {}", s))?,
        ),
        String::from(
            *v.get(1)
                .ok_or_else(|| anyhow!("Environment variable has no key: {}", s))?,
        ),
    ))
}
