use anyhow::{bail, Context, Result};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

pub struct Cache {
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
        Ok(Cache { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get the cache stamp with the given name.
    pub fn stamp(&self, name: impl AsRef<str>) -> Stamp {
        let name = name.as_ref();
        assert!(!name.contains('/'));
        assert!(!name.contains('\\'));
        Stamp {
            path: self.root().join("stamps").join(name),
        }
    }
}

/// A cache stamp.
///
/// Used to cache the positive results of existence checks, e.g. is the
/// `wasm32-wasi` target installed for a given Rust toolchain?
pub struct Stamp {
    path: PathBuf,
}

impl Stamp {
    /// Does the cache stamp file exist, and therefore we already know its
    /// associated check has already been performed on an earlier run and it
    /// succeeded? E.g. we already ensured that the `wasm32-wasi` target is
    /// installed.
    ///
    /// Note that this *can* be a false positive, since the user could have, for
    /// example, deleted the `wasm32-wasi` target through `rustup`.
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// Given that we know the stamp's associated check succeeded, or did the
    /// work necessary to ensure it will succeed on further checks, create the
    /// cached stamp file.
    pub fn create(self) -> Result<()> {
        let stamps_dir = self.path.parent().unwrap();
        fs::create_dir_all(stamps_dir).with_context(|| {
            format!("failed to create stamp directory: {}", stamps_dir.display())
        })?;
        File::create(&self.path)
            .with_context(|| format!("failed to create stamp file: {}", self.path.display()))?;
        Ok(())
    }

    /// If the cache stamp does not exist, invoke `f` to ensure its result, and
    /// then write the cache stamp file.
    ///
    /// `f` should return `Ok` if the cached check succeeded, and therefore the
    /// cache stamp file should be written so we don't try and re-ensure it next
    /// time, for example if the `wasm32-wasi` target was installed correctly.
    ///
    /// `f` should return `Err` if the cached check failed, and therefore should
    /// *not* write the cache stamp file so that we do attempt to ensure it
    /// again next time, for example if we failed to install the `wasm32-wasi`
    /// target.
    pub fn ensure(self, f: impl FnOnce() -> Result<()>) -> Result<()> {
        if self.exists() {
            return Ok(());
        }

        f()?;
        self.create()
    }
}
