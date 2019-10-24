use crate::Cache;
use anyhow::Result;
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
}
