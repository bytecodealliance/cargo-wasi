use crate::cache::Cache;
use crate::config::Config;
use crate::utils::CommandExt;
use anyhow::{bail, Context, Result};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

mod cache;
mod config;
mod utils;

pub fn main() {
    // See comments in `rmain` around `*_RUNNER` for why this exists here.
    if env::var("__CARGO_WASI_RUNNER_SHIM").is_ok() {
        let args = env::args().skip(1).collect();
        println!(
            "{}",
            serde_json::to_string(&CargoMessage::RunWithArgs { args }).unwrap(),
        );
        return;
    }

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
    let mut args = env::args_os().skip(2);
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
    cargo.arg(match subcommand {
        Subcommand::Build => "build",
        Subcommand::Check => "check",
        Subcommand::Fix => "fix",
        Subcommand::Test => "test",
        Subcommand::Bench => "bench",
        Subcommand::Run => "run",
    });

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

    // If Cargo actually executes a wasm file, we don't want it to. We need to
    // postprocess wasm files (wasm-opt, wasm-bindgen, etc). As a result we will
    // actually postprocess wasm files after the build. To work around this we
    // could pass `--no-run` for `test`/`bench`, but there's unfortunately no
    // equivalent for `run`. Additionally we want to learn what arguments Cargo
    // parsed to pass to each wasm file.
    //
    // To solve this all we do a bit of a switcharoo. We say that *we* are the
    // runner, and our binary is configured to simply print a json message at
    // the beginning. We'll slurp up these json messages and then actually
    // execute everything at the end.
    //
    // Also note that we check here before we actually build that a runtime is
    // present, notably `wasmtime`.
    match subcommand {
        Subcommand::Run | Subcommand::Bench | Subcommand::Test => {
            if which::which("wasmtime").is_err() {
                let mut msg = format!(
                    "failed to find `wasmtime` in $PATH, you'll want to \
                     install `wasmtime` before running this command\n"
                );
                if cfg!(unix) {
                    msg.push_str("you can also install through a shell:\n\n");
                    msg.push_str("\tcurl https://wasmtime.dev/install.sh -sSf | bash\n");
                } else {
                    msg.push_str("you can also install through the installer:\n\n");
                    msg.push_str("\thttps://github.com/CraneStation/wasmtime/releases/download/dev/wasmtime-dev-x86_64-windows.msi\n");
                }
                bail!("{}", msg);
            }
            cargo.env("__CARGO_WASI_RUNNER_SHIM", "1");
            cargo.env("CARGO_TARGET_WASM32_WASI_RUNNER", env::current_exe()?);
        }

        Subcommand::Build | Subcommand::Check | Subcommand::Fix => {}
    }

    install_wasi_target(&config)?;
    let build = execute_cargo(&mut cargo, &config)?;
    for (wasm, profile, fresh) in build.wasms.iter() {
        // Cargo will always overwrite our `wasm` above with its own internal
        // cache. It's internal cache largely uses hard links.
        //
        // If `fresh` is *false*, then Cargo just built `wasm` and we need to
        // process it. If `fresh` is *true*, then we may have previously
        // processed it. If our previous processing was successful the output
        // was placed at `*.wasi.wasm`, so we use that to overwrite the
        // `*.wasm` file. In the process we also create a `*.rustc.wasm` for
        // debugging.
        //
        // Note that we remove files before renaming and such to ensure that
        // we're not accidentally updating the wrong hard link and such.
        let temporary_rustc = wasm.with_extension("rustc.wasm");
        let temporary_wasi = wasm.with_extension("wasi.wasm");

        drop(fs::remove_file(&temporary_rustc));
        fs::rename(wasm, &temporary_rustc)?;
        if !*fresh || !temporary_wasi.exists() {
            // If we found `wasm-bindgen` as a dependency when building then
            // automatically execute the `wasm-bindgen` CLI, otherwise just process
            // using normal `walrus` commands.
            let result = match &build.wasm_bindgen {
                Some(version) => run_wasm_bindgen(
                    &temporary_wasi,
                    &temporary_rustc,
                    profile,
                    version,
                    &build,
                    &config,
                ),
                None => process_wasm(&temporary_wasi, &temporary_rustc, profile, &build, &config),
            };
            result.with_context(|| {
                format!("failed to process wasm at `{}`", temporary_rustc.display())
            })?;
        }
        drop(fs::remove_file(&wasm));
        fs::hard_link(&temporary_wasi, &wasm)
            .or_else(|_| fs::copy(&temporary_wasi, &wasm).map(|_| ()))?;
    }

    for run in build.runs.iter() {
        config.status("Running", &format!("`{}`", run.join(" ")));
        Command::new("wasmtime")
            .arg("--")
            .args(run.iter())
            .run()
            .map_err(|e| utils::hide_normal_process_exit(e, config))?;
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

/// Installs the `wasm32-wasi` target into our global cache.
fn install_wasi_target(config: &Config) -> Result<()> {
    // We'll make a stamp file when we verify that wasm32-wasi is installed to
    // accelerate future checks. If that file exists, we're good to go.
    //
    // Note that we account for `$RUSTUP_TOOLCHAIN` if it exists to ensure that
    // if you're moving across toolchains we always make sure that wasi is
    // installed.
    let stamp_name = "wasi-target-installed".to_string()
        + &env::var("RUSTUP_TOOLCHAIN").unwrap_or("".to_string());
    config.cache().stamp(stamp_name).ensure(|| {
        // Ok we need to actually check since this is perhaps the first time we've
        // ever checked. Let's ask rustc what its sysroot is and see if it has a
        // wasm32-wasi folder.
        let sysroot = Command::new("rustc")
            .arg("--print")
            .arg("sysroot")
            .capture_stdout()?;
        let sysroot = Path::new(sysroot.trim());
        if sysroot.join("lib/rustlib/wasm32-wasi").exists() {
            return Ok(());
        }

        // ... and that doesn't exist, so we need to install it! If we're not a
        // rustup toolchain then someone else has to figure out how to install the
        // wasi target, otherwise we delegate to rustup.
        if env::var_os("RUSTUP_TOOLCHAIN").is_none() {
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
        Ok(())
    })
}

#[derive(Default, Debug)]
struct CargoBuild {
    // The version of `wasm-bindgen` used in this build, if any.
    wasm_bindgen: Option<String>,
    // The `*.wasm` artifacts we found during this build, in addition to the
    // profile that they were built with and whether or not it was `fresh`
    // during this build.
    wasms: Vec<(PathBuf, Profile, bool)>,
    // executed commands as part of the cargo build
    runs: Vec<Vec<String>>,
    // Configuration we found in the `Cargo.toml` workspace manifest for these
    // builds.
    manifest_config: ManifestConfig,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
struct Profile {
    opt_level: String,
    debuginfo: Option<u32>,
    test: bool,
}

#[derive(serde::Deserialize, Debug, Default)]
#[serde(rename_all = "kebab-case")]
struct ManifestConfig {
    wasm_opt: Option<bool>,
    wasm_name_section: Option<bool>,
    wasm_producers_section: Option<bool>,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(tag = "reason", rename_all = "kebab-case")]
enum CargoMessage {
    CompilerArtifact {
        filenames: Vec<String>,
        package_id: String,
        profile: Profile,
        fresh: bool,
    },
    BuildScriptExecuted,
    RunWithArgs {
        args: Vec<String>,
    },
}

impl CargoBuild {
    fn enable_name_section(&self, profile: &Profile) -> bool {
        profile.debuginfo.is_some() || self.manifest_config.wasm_name_section.unwrap_or(true)
    }

    fn enable_producers_section(&self, profile: &Profile) -> bool {
        profile.debuginfo.is_some() || self.manifest_config.wasm_producers_section.unwrap_or(true)
    }
}

/// Executes the `cargo` command, reading all of the JSON that pops out and
/// parsing that into a `CargoBuild`.
fn execute_cargo(cargo: &mut Command, config: &Config) -> Result<CargoBuild> {
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
    utils::check_success(&cargo, &status, &[], &[])
        .map_err(|e| utils::hide_normal_process_exit(e, config))?;

    let mut build = CargoBuild::default();
    for line in json.lines() {
        match serde_json::from_str(line) {
            Ok(CargoMessage::CompilerArtifact {
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
            Ok(CargoMessage::RunWithArgs { args }) => build.runs.push(args),
            Ok(CargoMessage::BuildScriptExecuted) => {}
            Err(e) => bail!("failed to parse {}: {}", line, e),
        }
    }

    #[derive(serde::Deserialize)]
    struct CargoMetadata {
        workspace_root: String,
    }

    #[derive(serde::Deserialize)]
    struct CargoManifest {
        package: CargoPackage,
    }

    #[derive(serde::Deserialize)]
    struct CargoPackage {
        metadata: Option<ManifestConfig>,
    }

    let metadata = Command::new("cargo")
        .arg("metadata")
        .arg("--no-deps")
        .arg("--format-version=1")
        .capture_stdout()?;
    let metadata = serde_json::from_str::<CargoMetadata>(&metadata)
        .context("failed to deserialize `cargo metadata`")?;
    let manifest = Path::new(&metadata.workspace_root).join("Cargo.toml");
    let toml = fs::read_to_string(&manifest)
        .context(format!("failed to read manifest: {}", manifest.display()))?;
    let toml = toml::from_str::<CargoManifest>(&toml).context(format!(
        "failed to deserialize as TOML: {}",
        manifest.display()
    ))?;

    if let Some(meta) = toml.package.metadata {
        build.manifest_config = meta;
    }

    Ok(build)
}

/// Process a wasm file that doesn't use `wasm-bindgen`, using `walrus` instead.
///
/// This will load up the module and do things like:
///
/// * Unconditionally demangle all Rust function names.
/// * Use `profile` to optionally drop debug information
fn process_wasm(
    wasm: &Path,
    temp: &Path,
    profile: &Profile,
    build: &CargoBuild,
    config: &Config,
) -> Result<()> {
    config.verbose(|| {
        config.status("Processing", &temp.display().to_string());
    });

    let mut module = walrus::ModuleConfig::new()
        // If the `debuginfo` is configured then we leave in the debuginfo
        // sections.
        .generate_dwarf(profile.debuginfo.is_some())
        .generate_name_section(build.enable_name_section(profile))
        .generate_producers_section(build.enable_producers_section(profile))
        .strict_validate(false)
        .parse_file(temp)?;

    // Demangle everything so it's got a more readable name since there's
    // no real need to mangle the symbols in wasm.
    for func in module.funcs.iter_mut() {
        if let Some(name) = &mut func.name {
            if let Ok(sym) = rustc_demangle::try_demangle(name) {
                *name = sym.to_string();
            }
        }
    }

    run_wasm_opt(wasm, &module.emit_wasm(), profile, build, config)?;
    Ok(())
}

/// Executes `wasm-bindgen` over the `wasm` file provided, using `profile` to
/// guide the flags to pass to `wasm-bindgen`.
///
/// If `$WASM_BINDGEN` is set we'll unconditionally use that, otherwise we'll
/// fall back to either downloading a precompiled version of `bindgen_version`
/// or installing `bindgen_version` via `cargo install`.
fn run_wasm_bindgen(
    wasm: &Path,
    temp: &Path,
    profile: &Profile,
    bindgen_version: &str,
    build: &CargoBuild,
    config: &Config,
) -> Result<()> {
    let tempdir = tempfile::TempDir::new_in(wasm.parent().unwrap())
        .context("failed to create temporary directory")?;
    let (wasm_bindgen, cache_wasm_bindgen) = config.get_wasm_bindgen(bindgen_version);

    let mut cmd = Command::new(&wasm_bindgen);
    cmd.arg(temp);
    if profile.debuginfo.is_some() {
        cmd.arg("--keep-debug");
    }
    cmd.arg("--out-dir").arg(tempdir.path());
    cmd.arg("--out-name").arg("foo");
    cmd.env("WASM_INTERFACE_TYPES", "1");
    if !build.enable_name_section(profile) {
        cmd.arg("--remove-name-section");
    }
    if !build.enable_producers_section(profile) {
        cmd.arg("--remove-producers-section");
    }

    run_or_download(
        wasm_bindgen.as_ref(),
        &cache_wasm_bindgen,
        &mut cmd,
        config,
        || install_wasm_bindgen(bindgen_version, wasm_bindgen.as_ref(), config),
    )?;

    // note that we explicitly don't run `wasm-opt` right now since that will
    // interfere with the current interface-types implementation
    fs::copy(tempdir.path().join("foo.wasm"), wasm)?;
    Ok(())
}

/// Installs `wasm-bindgen` executable to `path` with the version `version`.
///
/// This will download from the network or do a very long compile locally.
fn install_wasm_bindgen(version: &str, path: &Path, config: &Config) -> Result<()> {
    // Downloads a precompiled tarball for `target` and places it in `path`.
    let download_precompiled = |target: &str| {
        let mut url = "https://github.com/rustwasm/wasm-bindgen/releases/download/".to_string();
        url.push_str(version);
        url.push_str("/wasm-bindgen-");
        url.push_str(version);
        url.push_str("-");
        url.push_str(target);
        url.push_str(".tar.gz");
        download(
            &url,
            &format!("precompiled wasm-bindgen v{}", version),
            path,
            config,
        )
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
    let parent = path.parent().unwrap();
    let filename = path.file_name().unwrap();
    config.status("Installing", &format!("wasm-bindgen v{}", version));
    let path = env::var_os("PATH").unwrap_or_default();
    let mut path = env::split_paths(&path).collect::<Vec<_>>();
    path.push(parent.join("bin"));
    let path = env::join_paths(&path)?;
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

fn run_wasm_opt(
    wasm: &Path,
    bytes: &[u8],
    profile: &Profile,
    build: &CargoBuild,
    config: &Config,
) -> Result<()> {
    // If debuginfo is enabled, automatically disable `wasm-opt`. It will mess
    // up dwarf debug information currently, so we can't run it.
    //
    // Additionally if no optimizations are enabled, no need to run `wasm-opt`,
    // we're not optimizing.
    if profile.debuginfo.is_some() || profile.opt_level == "0" {
        fs::write(wasm, bytes)?;
        return Ok(());
    }

    // Allow explicitly disabling wasm-opt via `Cargo.toml`.
    if build.manifest_config.wasm_opt == Some(false) {
        fs::write(wasm, bytes)?;
        return Ok(());
    }

    config.status("Optimizing", "with wasm-opt");
    let tempdir = tempfile::TempDir::new_in(wasm.parent().unwrap())
        .context("failed to create temporary directory")?;
    let (wasm_opt, cached_wasm_opt) = config.get_wasm_opt();

    let input = tempdir.path().join("input.wasm");
    fs::write(&input, &bytes)?;
    let mut cmd = Command::new(&wasm_opt);
    cmd.arg(&input);
    cmd.arg(format!("-O{}", profile.opt_level));
    cmd.arg("-o").arg(wasm);

    if build.enable_name_section(profile) {
        cmd.arg("--debuginfo");
    } else {
        cmd.arg("--strip-debug");
    }

    if !build.enable_producers_section(profile) {
        cmd.arg("--strip-producers");
    }

    run_or_download(
        wasm_opt.as_ref(),
        cached_wasm_opt.as_ref(),
        &mut cmd,
        config,
        || install_wasm_opt(wasm_opt.as_ref(), config),
    )
    .context("`wasm-opt` failed to execute")?;
    Ok(())
}

/// Attempts to execute `cmd` which is executing `requested`.
///
/// If the execution fails because `requested` isn't found *and* `requested` is
/// the same as the `cache` path provided, then `download` is invoked to
/// download the tool and then we re-execute `cmd` after the download has
/// finished.
///
/// Additionally nice diagnostics and such are printed along the way.
fn run_or_download(
    requested: &Path,
    cache: &Path,
    cmd: &mut Command,
    config: &Config,
    download: impl FnOnce() -> Result<()>,
) -> Result<()> {
    // NB: this is explicitly set up so that, by default, we simply execute the
    // command and assume that it exists. That should ideally avoid a few extra
    // syscalls to detect "will things work?"
    config.verbose(|| {
        if requested.exists() {
            config.status("Running", &format!("{:?}", cmd));
        }
    });

    let err = match cmd.run() {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };
    let any_not_found = err.chain().any(|e| {
        if let Some(err) = e.downcast_ref::<io::Error>() {
            return err.kind() == io::ErrorKind::NotFound;
        }
        false
    });

    // This may have failed for some reason other than `NotFound`, in which case
    // it's a legitimate error. Additionally `requested` may not actually be a
    // path that we download, in which case there's also nothing that we can do.
    if !any_not_found || requested != cache {
        return Err(err);
    }

    download()?;
    config.verbose(|| {
        config.status("Running", &format!("{:?}", cmd));
    });
    cmd.run()
}

fn install_wasm_opt(path: &Path, config: &Config) -> Result<()> {
    let tag = "version_89";
    let target = if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
        "x86_64-linux"
    } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
        "x86_64-apple-darwin"
    } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
        "x86_64-windows"
    } else {
        bail!(
            "no precompiled binaries of `wasm-opt` are available for this \
             platform, you'll want to set `$WASM_OPT` to a preinstalled \
             `wasm-opt` command or disable via `wasm-opt = false` in \
             your manifest"
        )
    };

    let mut url = "https://github.com/WebAssembly/binaryen/releases/download/".to_string();
    url.push_str(tag);
    url.push_str("/binaryen-");
    url.push_str(tag);
    url.push_str("-");
    url.push_str(target);
    url.push_str(".tar.gz");

    download(&url, &format!("precompiled wasm-opt {}", tag), path, config)
}

fn download(url: &str, name: &str, path: &Path, config: &Config) -> Result<()> {
    // Globally lock ourselves downloading things to coordinate with any other
    // instances of `cargo-wasi` doing a download. This is a bit coarse, but it
    // gets the job done. Additionally if someone else does the download for us
    // then we can simply return.
    let _flock = utils::flock(&config.cache().root().join("downloading"));
    if path.exists() {
        return Ok(());
    }

    // Ok, let's actually do the download
    let parent = path.parent().unwrap();
    let filename = path.file_name().unwrap();
    config.status("Downloading", name);
    config.verbose(|| config.status("Get", &url));

    let response = reqwest::get(url).context(format!("failed to fetch {}", url))?;
    if !response.status().is_success() {
        bail!(
            "failed to get successful response from {}: {}",
            url,
            response.status()
        );
    }
    (|| -> Result<()> {
        fs::create_dir_all(parent)
            .context(format!("failed to create directory `{}`", parent.display()))?;

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
    })()
    .context(format!("failed to extract tarball from {}", url))
}
