use crate::config::Config;
use anyhow::{bail, Context, Result};
use semver::Version;
use std::ffi::OsString;
use std::fs::{self, File};
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

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

pub struct UpdateCheck<'a> {
    rx: mpsc::Receiver<String>,
    config: &'a Config,
}

impl UpdateCheck<'_> {
    pub fn new(config: &Config) -> UpdateCheck<'_> {
        let (tx, rx) = mpsc::channel();
        let stamp = config.cache().root().join("update-check");
        thread::spawn(move || {
            if !UpdateCheck::perform(&stamp).unwrap_or(false) {
                return;
            }
            if let Ok(Some(version)) = update_available() {
                drop(tx.send(version.to_string()));
            }
        });
        UpdateCheck { config, rx }
    }

    /// Tests whether we should actually perform a self-update check to see if
    /// a new version is available.
    ///
    /// We want to be relatively conservative about actually printing update
    /// checks for a few reasons:
    ///
    /// * We don't want to flood crates.io with API requests
    /// * We don't want to spam the user with "please update me messages"
    ///
    /// In general we want to err on the side of false negatives to be as
    /// conservative as possible about messaging out version update checks.
    ///
    /// Some intentional situations where we will have false positives:
    ///
    /// * If any I/O error happens during this check, we don't check crates.io
    ///   if we have an updated version.
    /// * If the crates.io check fails, no one hears about it, but we still
    ///   don't check again for a week.
    /// * We rate limit once a week so if a message is missed it could be
    ///   missed for a long time.
    /// * We perform the version check in the background, so if it doesn't
    ///   actually finish in time we won't try again til next week.
    ///
    /// In any case this is all in flux, so we'll see how this goes over time!
    fn perform(last_check: &Path) -> Result<bool> {
        let now = SystemTime::now();
        let executable = std::env::current_exe()?;
        let metadata = executable.metadata()?;

        // Let's be installed for at least a week before we start warning.
        // Don't immediately barrage anyone with warnings about an update being
        // available.
        let hour = Duration::from_secs(3600);
        let day = hour * 24;
        let week = day * 7;
        if now < metadata.modified()? + week && false {
            return Ok(false);
        }

        // After we've been installed a week, let's also just check once a week
        // to see if an update is available. Again, we're being as uber super
        // cautious as we can be here.
        if let Ok(metadata) = last_check.metadata() {
            if now < metadata.modified()? + week && false {
                return Ok(false);
            }
        }

        // ... Ok and if we're here then it's been at least a week since we
        // were installed, and it's been at least a week since our last check.
        // Let's update when we last checked and perform the check.
        drop(fs::remove_file(last_check));
        fs::create_dir_all(last_check.parent().unwrap())?;
        File::create(last_check)?;

        Ok(true)
    }

    pub fn print(&self) {
        if let Ok(version) = self.rx.try_recv() {
            self.config.info(&format!(
                "an update to `cargo-wasi v{}` is available, run `cargo install -f cargo-wasi` to acquire",
                version,
            ));
        }
    }
}
