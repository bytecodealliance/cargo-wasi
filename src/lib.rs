use crate::cache::Cache;
use crate::config::Config;
use crate::utils::CommandExt;
use anyhow::{bail, Context, Result};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

mod cache;
mod config;
mod utils;

pub fn main() {
    let mut config = Config::new();
    match rmain(&mut config) {
        Ok(()) => {}
        Err(e) => {
            config.print_error(&e);
            std::process::exit(1);
        }
    }
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

fn rmain(config: &mut Config) -> Result<()> {
    config.load_cache()?;

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
        if let Some(arg) = arg.to_str() {
            if arg.starts_with("--verbose") || arg.starts_with("-v") {
                config.set_verbose(true);
            }
        }

        cargo.arg(arg);
    }

    install_wasi_target(&config)?;
    let build = execute_cargo(&mut cargo, &config)?;
    for (wasm, profile, fresh) in build.wasms.iter() {
        drop(fresh); // TODO: should handle this
        let result = match &build.wasm_bindgen {
            Some(version) => run_wasm_bindgen(wasm, profile, version, &config),
            None => process_wasm(wasm, profile, &config),
        };
        result.with_context(|| format!("failed to process wasm at `{}`", wasm.display()))?;
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

fn install_wasi_target(config: &Config) -> Result<()> {
    // We'll make a stamp file when we verify that wasm32-wasi is installed to
    // accelerate future checks. If that file exists, we're good to go.
    //
    // Note that we account for `$RUSTUP_TOOLCHAIN` if it exists to ensure that
    // if you're moving across toolchains we always make sure that wasi is
    // installed.
    let mut stamp_name = "wasi-target-installed".to_string();
    if let Ok(s) = std::env::var("RUSTUP_TOOLCHAIN") {
        stamp_name.push_str(&s);
    }
    let stamp = config.cache().root().join(&stamp_name);
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
    let _lock = utils::flock(&config.cache().root().join("rustup-lock"));

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
    // profile that they were built with and whether or not it was `fresh`
    // during this build.
    wasms: Vec<(PathBuf, Profile, bool)>,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct Profile {
    opt_level: String,
    debuginfo: Option<u32>,
    test: bool,
}

fn execute_cargo(cargo: &mut Command, config: &Config) -> Result<CargoBuild> {
    #[derive(serde::Deserialize)]
    #[serde(tag = "reason", rename_all = "kebab-case")]
    enum Message {
        CompilerArtifact {
            filenames: Vec<String>,
            package_id: String,
            profile: Profile,
            fresh: bool,
        },
        BuildScriptExecuted,
    }
    config.verbose(|| config.status("Running", &format!("{:?}", cargo)));
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
                fresh,
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
                        build.wasms.push((file, profile.clone(), fresh));
                    }
                }
            }
            Ok(Message::BuildScriptExecuted) => {}
            Err(e) => bail!("failed to parse {}: {}", line, e),
        }
    }

    Ok(build)
}

fn process_wasm(wasm: &Path, profile: &Profile, config: &Config) -> Result<()> {
    config.verbose(|| {
        config.status("Processing", &wasm.display().to_string());
    });

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

fn run_wasm_bindgen(
    wasm: &Path,
    profile: &Profile,
    bindgen_version: &str,
    config: &Config,
) -> Result<()> {
    let tempdir = tempfile::TempDir::new().context("failed to create temporary directory")?;
    let cache_wasm_bindgen = config
        .cache()
        .root()
        .join("wasm-bindgen")
        .join(bindgen_version)
        .join("wasm-bindgen")
        .with_extension(std::env::consts::EXE_EXTENSION);

    let wasm_bindgen =
        std::env::var_os("WASM_BINDGEN").unwrap_or(cache_wasm_bindgen.clone().into());

    let mut cmd = Command::new(&wasm_bindgen);
    cmd.arg(wasm);
    if profile.debuginfo.is_some() {
        cmd.arg("--keep-debug");
    }
    cmd.arg("--out-dir").arg(tempdir.path());
    cmd.arg("--out-name").arg("foo");
    cmd.env("WASM_INTERFACE_TYPES", "1");

    config.verbose(|| {
        config.status("Running", &format!("{:?}", cmd));
    });

    if let Err(e) = cmd.run() {
        let any_not_found = e.chain().any(|e| {
            if let Some(err) = e.downcast_ref::<io::Error>() {
                return err.kind() == io::ErrorKind::NotFound;
            }
            false
        });
        if any_not_found && wasm_bindgen == cache_wasm_bindgen {
            install_wasm_bindgen(bindgen_version, wasm_bindgen.as_ref(), config)?;
            cmd.run()?;
        } else {
            return Err(e);
        }
    }

    fs::copy(tempdir.path().join("foo.wasm"), wasm)?;

    Ok(())
}

fn install_wasm_bindgen(version: &str, path: &Path, config: &Config) -> Result<()> {
    let parent = path.parent().unwrap();
    let filename = path.file_name().unwrap();
    fs::create_dir_all(parent)
        .context(format!("failed to create directory `{}`", parent.display()))?;

    // Looks for `wasm-bindgen` in the compressed tarball that `response` has
    // and places it in `path`.
    let extract = |response: reqwest::Response| -> Result<()> {
        let decompressed = flate2::read::GzDecoder::new(response);
        let mut tar = tar::Archive::new(decompressed);
        for entry in tar.entries()? {
            let mut entry = entry?;
            if !entry.path()?.ends_with(filename) {
                continue;
            }
            entry.unpack(path)?;
            return Ok(());
        }

        bail!("failed to find {:?} in archive", filename);
    };

    // Downloads a precompiled tarball for `target` and places it in `path`.
    let download_precompiled = |target: &str| {
        let mut url = "https://github.com/rustwasm/wasm-bindgen/releases/download/".to_string();
        url.push_str(version);
        url.push_str("/wasm-bindgen-");
        url.push_str(version);
        url.push_str("-");
        url.push_str(target);
        url.push_str(".tar.gz");

        config.status(
            "Downloading",
            &format!("precompiled wasm-bindgen v{}", version),
        );
        config.verbose(|| {
            config.status("Get", &url);
        });

        let response = reqwest::get(&url).context(format!("failed to fetch {}", url))?;
        if !response.status().is_success() {
            bail!(
                "failed to get successful response from {}: {}",
                url,
                response.status()
            );
        }
        extract(response).context(format!("failed to extract tarball from {}", url))
    };

    // First check for precompiled artifacts
    if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        return download_precompiled("x86_64-unknown-linux-musl");
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        return download_precompiled("x86_64-apple-darwin");
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        return download_precompiled("x86_64-pc-windows-msvc");
    }

    // ... otherwise fall back to `cargo install`. Note that we modify `PATH`
    // here to suppress the warning that cargo emits about adding it to PATH.
    config.status("Installing", &format!("wasm-bindgen v{}", version));
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut path = std::env::split_paths(&path).collect::<Vec<_>>();
    path.push(parent.join("bin"));
    let path = std::env::join_paths(&path)?;
    Command::new("cargo")
        .arg("install")
        .arg("wasm-bindgen-cli")
        .arg("--version")
        .arg(format!("={}", version))
        .arg("--root")
        .arg(parent)
        .arg("--bin")
        .arg("wasm-bindgen")
        .env("PATH", &path)
        .run()?;

    fs::rename(parent.join("bin").join(filename), parent.join(filename))?;
    Ok(())
}
