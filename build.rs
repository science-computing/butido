use anyhow::Result;
use vergen::{Config, vergen};


fn main() -> Result<()> {
    let info = git_info::get();
    let mut config = Config::default();

    *config.git_mut().semver_dirty_mut() = match info.dirty {
        Some(true) => Some("-dirty"),
        _ => None,
    };

    vergen(config)
}
