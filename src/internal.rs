use crate::config::Config;
use anyhow::{bail, Context, Result};
use std::ffi::OsString;
use std::fs;

pub fn main(args: &[OsString], config: &Config) -> Result<()> {
    match args.get(0).and_then(|s| s.to_str()) {
        Some("clean") => clean(config),
        Some(other) => bail!("unsupported `self` command: {}", other),
        None => bail!("`self` command must be followed by `clean`"),
    }
}

fn clean(config: &Config) -> Result<()> {
    let path = config.cache().all_versions_root();
    config.status("Removing", &path.display().to_string());
    if path.exists() {
        fs::remove_dir_all(path).context(format!("failed to remove `{}`", path.display()))?;
    }
    Ok(())
}
