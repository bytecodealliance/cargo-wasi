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
        Subcommand::Fix => {
            cargo.arg("fix");
        }
        Subcommand::Test => {
            cargo.arg("test");
            cargo.arg("--no-run");
        }
        Subcommand::Bench => {
            cargo.arg("bench");
            cargo.arg("--no-run");
        }
        Subcommand::Run => {
            cargo.arg("build");
        }
    }

    // TODO: figure out when these flags are already passed to `cargo` and skip
    // passing them ourselves.
    cargo.arg("--target").arg("wasm32-wasi");
    cargo.arg("--message-format").arg("json-render-diagnostics");
    for arg in args {
        cargo.arg(arg);
    }
    let build = execute_cargo(&mut cargo)?;
    match &build.wasm_bindgen {
        Some(version) => run_wasm_bindgen(&build, version)?,
        None => process_wasms(&build)?,
    }

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

    // rustup is not itself synchronized across processes so at least attempt to
    // synchronize our own calls. This may not work and if it doesn't we tried,
    // this is largely opportunistic anyway.
    let _lock = utils::flock(&cache.root().join("rustup-lock"));

    Command::new("rustup")
        .arg("target")
        .arg("add")
        .arg("wasm32-wasi")
        .run()?;
    File::create(&stamp).context("failed to create stamp file")?;
    Ok(())
}

#[derive(Default, Debug)]
struct CargoBuild {
    // The version of `wasm-bindgen` used in this build, if any.
    wasm_bindgen: Option<String>,
    // The `*.wasm` artifacts we found during this build, in addition to the
    // profile that they were built with.
    wasms: Vec<(PathBuf, Profile)>,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct Profile {
    opt_level: String,
    debuginfo: Option<u32>,
    test: bool,
}

fn execute_cargo(cargo: &mut Command) -> Result<CargoBuild> {
    #[derive(serde::Deserialize)]
    #[serde(tag = "reason", rename_all = "kebab-case")]
    enum Message {
        CompilerArtifact {
            filenames: Vec<String>,
            package_id: String,
            profile: Profile,
        },
        BuildScriptExecuted,
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

    let mut build = CargoBuild::default();
    for line in json.lines() {
        match serde_json::from_str(line) {
            Ok(Message::CompilerArtifact {
                filenames,
                profile,
                package_id,
            }) => {
                let mut parts = package_id.split_whitespace();
                if parts.next() == Some("wasm-bindgen") {
                    if let Some(version) = parts.next() {
                        build.wasm_bindgen = Some(version.to_string());
                    }
                }
                for file in filenames {
                    let file = PathBuf::from(file);
                    if file.extension().and_then(|s| s.to_str()) == Some("wasm") {
                        build.wasms.push((file, profile.clone()));
                    }
                }
            }
            Ok(Message::BuildScriptExecuted) => {}
            Err(e) => bail!("failed to parse {}: {}", line, e),
        }
    }

    Ok(build)
}

/// Removes any `.debug_*` sections in wasm files produced during a build that
/// shouldn't have them.
///
/// If debuginfo is disabled during a build then the standard library's
/// debuginfo will still be present in the final executable. If debuginfo is
/// disabled though this is generally wasted space, so let's remove that
/// debuginfo.
fn process_wasms(build: &CargoBuild) -> Result<()> {
    for (wasm, profile) in build.wasms.iter() {
        process_wasm(wasm, profile)
            .with_context(|| format!("failed to process wasm at `{}`", wasm.display()))?;
    }
    return Ok(());

    fn process_wasm(wasm: &Path, profile: &Profile) -> Result<()> {
        let mut module = walrus::ModuleConfig::new()
            // If the `debuginfo` is configured then we leave in the debuginfo
            // sections.
            .generate_dwarf(profile.debuginfo.is_some())
            .generate_name_section(true)
            .strict_validate(false)
            .parse_file(wasm)?;

        // Demangle everything so it's got a more readable name since there's
        // no real need to mangle the symbols in wasm.
        for func in module.funcs.iter_mut() {
            if let Some(name) = &mut func.name {
                if let Ok(sym) = rustc_demangle::try_demangle(name) {
                    *name = sym.to_string();
                }
            }
        }

        std::fs::write(wasm, module.emit_wasm())?;
        Ok(())
    }
}

fn run_wasm_bindgen(build: &CargoBuild, version: &str) -> Result<()> {
    drop(build);
    panic!("not implemented");
}
