use crate::cache::Cache;
use crate::utils::CommandExt;
use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

mod cache;
mod utils;

pub fn main() {
    let err = match rmain() {
        Ok(()) => return,
        Err(e) => e,
    };
    let mut shell = StandardStream::stderr(ColorChoice::Auto);
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
    std::process::exit(1);
}

#[derive(Debug)]
enum Subcommand {
    Build,
    Run,
    Test,
    Bench,
    Check,
    Fix,
}

fn rmain() -> Result<()> {
    // skip the current executable and the `wasi` inserted by Cargo
    let mut args = std::env::args_os().skip(2);
    let subcommand = args.next().and_then(|s| s.into_string().ok());
    let subcommand = match subcommand.as_ref().map(|s| s.as_str()) {
        Some("build") => Subcommand::Build,
        Some("run") => Subcommand::Run,
        Some("test") => Subcommand::Test,
        Some("bench") => Subcommand::Bench,
        Some("check") => Subcommand::Check,
        Some("fix") => Subcommand::Fix,
        Some("version") | Some("-V") | Some("--version") => {
            let git_info = match option_env!("GIT_INFO") {
                Some(s) => format!(" ({})", s),
                None => String::new(),
            };
            println!("cargo-wasi {}{}", env!("CARGO_PKG_VERSION"), git_info);
            std::process::exit(0);
        }
        _ => print_help(),
    };

    let cache = Cache::new()?;
    install_wasi_target(&cache)?;

    let mut cargo = Command::new("cargo");
    match subcommand {
        Subcommand::Build => {
            cargo.arg("build");
        }
        Subcommand::Check => {
            cargo.arg("check");
        }
        s => panic!("unimplemented subcommand {:?}", s),
    }

    // TODO: figure out when these flags are already passed to `cargo` and skip
    // passing them ourselves.
    cargo.arg("--target").arg("wasm32-wasi");
    cargo.arg("--message-format").arg("json-render-diagnostics");
    for arg in args {
        cargo.arg(arg);
    }
    let wasms = execute_cargo(&mut cargo)?;
    println!("{:?}", wasms);

    Ok(())
}

fn print_help() -> ! {
    println!(
        "\
cargo-wasi
Compile and run a Rust crate for the wasm32-wasi target

USAGE:
    cargo wasi build [OPTIONS]
    cargo wasi run [OPTIONS]
    cargo wasi test [OPTIONS]
    cargo wasi bench [OPTIONS]
    cargo wasi check [OPTIONS]
    cargo wasi fix [OPTIONS]

All options accepted are the same as that of the corresponding `cargo`
subcommands. You can run `cargo wasi build -h` for more information to learn
about flags that can be passed to `cargo wasi build`, which mirrors the
`cargo build` command.
"
    );
    std::process::exit(0);
}

fn install_wasi_target(cache: &Cache) -> Result<()> {
    // We'll make a stamp file when we verify that wasm32-wasi is installed to
    // accelerate future checks. If that file exists, we're good to go.
    let stamp = cache.root().join("wasi-target-installed");
    if stamp.exists() {
        return Ok(());
    }

    // Ok we need to actually check since this is perhaps the first time we've
    // ever checked. Let's ask rustc what its sysroot is and see if it has a
    // wasm32-wasi folder.
    let sysroot = Command::new("rustc")
        .arg("--print")
        .arg("sysroot")
        .capture_stdout()?;
    let sysroot = Path::new(sysroot.trim());
    if sysroot.join("lib/rustlib/wasm32-wasi").exists() {
        File::create(&stamp).context("failed to create stamp file")?;
        return Ok(());
    }

    // ... and that doesn't exist, so we need to install it! If we're not a
    // rustup toolchain then someone else has to figure out how to install the
    // wasi target, otherwise we delegate to rustup.
    if std::env::var_os("RUSTUP_TOOLCHAIN").is_none() {
        bail!(
            "failed to find the `wasm32-wasi` target installed, and rustup \
             is also not detected, you'll need to be sure to install the \
             `wasm32-wasi` target before using this command"
        );
    }

    Command::new("rustup")
        .arg("target")
        .arg("add")
        .arg("wasm32-wasi")
        .run()?;
    File::create(&stamp).context("failed to create stamp file")?;
    Ok(())
}

fn execute_cargo(cargo: &mut Command) -> Result<Vec<PathBuf>> {
    #[derive(serde::Deserialize)]
    #[serde(tag = "reason", rename_all = "kebab-case")]
    enum Message {
        CompilerArtifact { filenames: Vec<String> },
    }
    let mut process = cargo
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to spawn `cargo`")?;
    let mut json = String::new();
    process
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut json)
        .context("failed to read cargo stdout into a json string")?;
    let status = process.wait().context("failed to wait on `cargo`")?;
    utils::check_success(&cargo, &status, &[], &[])?;

    let mut wasms = Vec::new();
    for line in json.lines() {
        match serde_json::from_str(line) {
            Ok(Message::CompilerArtifact { filenames }) => {
                for file in filenames {
                    let file = PathBuf::from(file);
                    if file.extension().and_then(|s| s.to_str()) == Some("wasm") {
                        wasms.push(file);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(wasms)
}
