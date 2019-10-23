use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub struct Cache {
    all_versions_root: PathBuf,
    root: PathBuf,
}

impl Cache {
    pub fn new() -> Result<Cache> {
        let all_versions_root = match dirs::cache_dir() {
            Some(root) => root.join("cargo-wasi"),
            None => match dirs::home_dir() {
                Some(home) => home.join(".cargo-wasi"),
                None => bail!("failed to find home directory, is $HOME set?"),
            },
        };
        let root = all_versions_root.join(env!("CARGO_PKG_VERSION"));
        fs::create_dir_all(&root)
            .with_context(|| format!("failed to create cache directory: {}", root.display()))?;
        Ok(Cache {
            all_versions_root,
            root,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}
