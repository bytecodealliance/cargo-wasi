use crate::Cache;
use anyhow::Result;
use std::path::PathBuf;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

pub struct Config {
    cache: Option<Cache>,
    verbose: bool,
    choice: ColorChoice,
}

impl Config {
    pub fn new() -> Config {
        Config {
            cache: None,
            verbose: false,
            choice: if atty::is(atty::Stream::Stderr) {
                ColorChoice::Auto
            } else {
                ColorChoice::Never
            },
        }
    }

    pub fn load_cache(&mut self) -> Result<()> {
        assert!(!self.cache.is_some());
        self.cache = Some(Cache::new()?);
        Ok(())
    }

    pub fn cache(&self) -> &Cache {
        self.cache.as_ref().expect("cache not loaded yet")
    }

    pub fn is_verbose(&self) -> bool {
        self.verbose
    }

    pub fn verbose(&self, f: impl FnOnce()) {
        if self.verbose {
            f();
        }
    }

    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    pub fn status(&self, name: &str, rest: &str) {
        let mut shell = StandardStream::stderr(self.choice);
        drop(shell.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true)));
        eprint!("{:>12}", name);
        drop(shell.reset());
        eprintln!(" {}", rest);
    }

    pub fn print_error(&self, err: &anyhow::Error) {
        if let Some(code) = crate::utils::normal_process_exit_code(err) {
            std::process::exit(code);
        }
        let mut shell = StandardStream::stderr(self.choice);
        drop(shell.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true)));
        eprint!("error");
        drop(shell.reset());
        eprintln!(": {}", err);
        for cause in err.chain().skip(1) {
            eprintln!("");
            drop(shell.set_color(ColorSpec::new().set_bold(true)));
            eprint!("Caused by");
            drop(shell.reset());
            eprintln!(":");
            eprintln!("    {}", cause.to_string().replace("\n", "\n    "));
        }
    }

    /// Returns the path to execute a tool, and the cache path where it should
    /// be downloaded to if unavailable.
    ///
    /// These are not necessarily the same path (e.g. if using overrides)!
    ///
    /// To override the path used for a tool, set an env var of the tool's name
    /// in uppercase with hyphens replaced with underscores to the desired
    /// path. For example, `WASM_BINDGEN=path/to/wasm-bindgen` to override the
    /// `wasm-bindgen` used, or `WASM_OPT=path/to/wasm-opt` for `wasm-opt`.  or
    /// the `cache` as the fallback.
    fn get_tool(&self, tool: &str, version: Option<&str>) -> (PathBuf, PathBuf) {
        let mut cache_path = self.cache().root().join(tool);
        if let Some(v) = version {
            cache_path.push(v);
            cache_path.push(tool)
        }
        cache_path.set_extension(env::consts::EXE_EXTENSION);

        if let Some(s) = std::env::var_os(tool.to_uppercase().replace("-", "_")) {
            (s.into(), cache_path)
        } else {
            (cache_path.clone(), cache_path)
        }
    }

    /// Get the path to our `wasm-bindgen` tool for the given version, and the
    /// cache path where it should be downloaded to if missing.
    ///
    /// These are not necessarily the same path (e.g. if using overrides)!
    ///
    /// Overridable via setting the `WASM_BINDGEN=path/to/wasm-bindgen` env var.
    pub fn get_wasm_bindgen(&self, version: &str) -> (PathBuf, PathBuf) {
        self.get_tool("wasm-bindgen", Some(version))
    }

    /// Get the path to our `wasm-opt`, and the cache path where it should be
    /// downloaded to if missing.
    ///
    /// These are not necessarily the same path (e.g. if using overrides)!
    ///
    /// Overridable via setting the `WASM_OPT=path/to/wasm-opt` env var.
    pub fn get_wasm_opt(&self) -> (PathBuf, PathBuf) {
        self.get_tool("wasm-opt", None)
    }
}
