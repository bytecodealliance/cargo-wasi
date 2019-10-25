use crate::config::Config;
use anyhow::{bail, Context, Result};
use semver::Version;
use std::ffi::OsString;
use std::fs;

pub fn main(args: &[OsString], config: &Config) -> Result<()> {
    match args.get(0).and_then(|s| s.to_str()) {
        Some("clean") => clean(config),
        Some("update-check") => update_check(config),
        Some(other) => bail!("unsupported `self` command: {}", other),
        None => bail!("`self` command must be followed by `clean` or `update-check`"),
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

fn update_check(config: &Config) -> Result<()> {
    config.status("Checking", "for the latest release");
    match update_available()? {
        Some(version) => {
            eprintln!("An update to version {} is available!", version);
            eprintln!("To upgrade from {} run:", env!("CARGO_PKG_VERSION"));
            eprintln!("");
            eprintln!("    cargo install cargo-wasi -f");
            eprintln!("");
        }
        None => eprintln!("cargo-wasi v{} is up-to-date", env!("CARGO_PKG_VERSION")),
    }
    Ok(())
}

fn update_available() -> Result<Option<Version>> {
    #[derive(serde::Deserialize)]
    struct Info {
        versions: Vec<CrateVersion>,
    }
    #[derive(serde::Deserialize)]
    struct CrateVersion {
        num: String,
    }

    let url = "https://crates.io/api/v1/crates/cargo-wasi";
    let mut response = crate::utils::get(url)?;
    let json = response
        .json::<Info>()
        .context(format!("failed to decode json from `{}`", url))?;
    let mut versions = json
        .versions
        .iter()
        .filter_map(|t| Version::parse(&t.num).ok());
    let me = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
    Ok(versions.find(|v| *v > me))
}
