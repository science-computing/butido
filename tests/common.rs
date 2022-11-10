use std::path::Path;
use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*;

const CONFIG: &str = include_str!("../config.toml");

pub fn setup_cwd<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
    let mut toml: toml::Value = toml::from_str(CONFIG)?;

    for (key, value) in [
        ("releases_root", "releases"),
        ("staging", "staging"),
        ("source_cache", "sources"),
        ("log_dir", "logs"),
    ] {
        let path = path.as_ref().join(value);
        std::fs::create_dir(&path)?;
        let value = toml::Value::String(path.display().to_string());
        let toml_key = toml
            .get_mut(key)
            .ok_or_else(|| format!("{} missing in configuration", key))?;
        *toml_key = value
    }

    std::fs::write(
        path.as_ref().join("config.toml"),
        toml::to_string_pretty(&toml)?,
    )?;

    std::fs::write(
        path.as_ref().join("pkg.toml"),
        ""
    )?;

    std::process::Command::new("git")
        .current_dir(path)
        .arg("init")
        .assert()
        .success();

    Ok(())
}
