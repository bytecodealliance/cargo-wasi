use anyhow::Context;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const TARGETS: &[(&str, &str)] = &[
    (
        "x86_64-apple-darwin",
        r#"all(target_arch = "x86_64", target_os = "macos")"#,
    ),
    (
        "x86_64-pc-windows-msvc",
        r#"all(target_arch = "x86_64", target_os = "windows")"#,
    ),
    (
        "x86_64-unknown-linux-musl",
        r#"all(target_arch = "x86_64", target_os = "linux")"#,
    ),
];

fn main() -> anyhow::Result<()> {
    let tmp = Path::new("tmp");
    drop(fs::remove_dir_all(&tmp));
    fs::create_dir_all(tmp.join("shim")).context("failed to create `tmp` dir")?;

    println!("Copying the shim into `tmp`...");
    cp_r("crates/cargo-wasi-shim".as_ref(), &tmp.join("shim"))?;

    let version = toml::from_str::<toml::Value>(&fs::read_to_string("Cargo.toml").unwrap())
        .unwrap()["package"]["version"]
        .as_str()
        .unwrap()
        .to_string();

    let krate_files_here = std::env::var("SKIP_EXTRACT").is_err();

    if krate_files_here {
        for (target, _) in TARGETS {
            let dir = PathBuf::from(format!("cargo-wasi-{}", target));
            let krate = dir
                .read_dir()
                .context(format!("failed to read {:?}", dir))?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .find(|e| e.extension().and_then(|s| s.to_str()) == Some("crate"))
                .expect("failed to find `*.crate`");
            println!("extracting {:?}", krate);
            let status = Command::new("tar")
                .arg("xf")
                .arg(krate.canonicalize().context("failed to canonicalize")?)
                .current_dir(&tmp)
                .status()
                .context("failed to spawn `tar`")?;
            if !status.success() {
                anyhow::bail!("tar extraction failed: {}", status);
            }
        }
    }

    println!("Rewriting shim manifest with `[patch]`");
    let manifest_path = tmp.join("shim/Cargo.toml");
    fs::remove_file(tmp.join("shim/build.rs")).context("failed to remove build script")?;
    let mut manifest = fs::read_to_string(&manifest_path)
        .context("failed to read manifest")?
        .replace("cargo-wasi-shim", "cargo-wasi");
    manifest.truncate(manifest.find("[features]").unwrap());
    manifest.push_str("\n");
    for (target, cfg) in TARGETS {
        manifest.push_str("[target.'cfg(");
        manifest.push_str(cfg);
        manifest.push_str(")'.dependencies]\n");
        manifest.push_str(&format!("cargo-wasi-exe-{} = \"={}\"\n", target, version));
    }
    manifest.push_str("[target.'cfg(not(any(");
    for (_, cfg) in TARGETS {
        manifest.push_str(cfg);
        manifest.push_str(",");
    }
    manifest.push_str(")))'.dependencies]\n");
    manifest.push_str(&format!("cargo-wasi-src = \"={}\"\n", version));
    manifest.push_str("[patch.crates-io]\n");
    for (target, _) in TARGETS {
        manifest.push_str(&format!(
            "cargo-wasi-exe-{} = {{ path = '../cargo-wasi-exe-{0}-{}' }}\n",
            target, version
        ));
    }
    println!("========= NEW MANIFEST ===============");
    println!("\t{}", manifest.replace("\n", "\n\t"));
    fs::write(&manifest_path, manifest).context("failed to write manifest")?;

    if krate_files_here {
        println!("Building the shim to make sure it works");
        let status = Command::new("cargo")
            .arg("build")
            .env("CARGO_TARGET_DIR", "target")
            .arg("--manifest-path")
            .arg(&manifest_path)
            .status()
            .context("failed to spawn `cargo`")?;
        if !status.success() {
            anyhow::bail!("cargo failed: {}", status);
        }
    }

    let do_publish = std::env::var("NO_DRY_RUN").is_ok();

    // Rename the main package to `cargo-wasi-src` and then publish it.
    if do_publish {
        println!("Publishing the `cargo-wasi-src` package");
        let manifest = fs::read_to_string("Cargo.toml").context("failed to read manifest")?;
        fs::write(
            "Cargo.toml",
            manifest.replace("name = \"cargo-wasi\"", "name = \"cargo-wasi-src\""),
        )?;
        let status = Command::new("cargo")
            .arg("publish")
            .arg("--no-verify")
            .arg("--allow-dirty")
            .status()
            .context("failed to spawn `cargo`")?;
        fs::write("Cargo.toml", manifest)?;
        if !status.success() {
            anyhow::bail!("cargo failed: {}", status);
        }
    }

    let mut entries = tmp
        .read_dir()?
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();
    entries.sort();
    for entry in entries {
        println!("publish {:?}", entry);
        let mut cmd = Command::new("cargo");
        cmd.arg("publish").current_dir(&entry);
        if !do_publish {
            cmd.arg("--dry-run");

            if entry.ends_with("tmp/shim") {
                println!("skipping `shim` since crates aren't published");
                continue;
            }
        } else {
            // give crates.io a chance to propagate the index change
            std::thread::sleep(std::time::Duration::from_secs(15));
        }
        let status = cmd.status().context("failed to spawn `cargo`")?;
        if !status.success() {
            anyhow::bail!("cargo failed: {}", status);
        }
    }

    Ok(())
}

fn cp_r(a: &Path, b: &Path) -> anyhow::Result<()> {
    for entry in a.read_dir().context("failed to read source directory")? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let b = b.join(entry.file_name());
            fs::create_dir_all(&b)?;
            cp_r(&entry.path(), &b)?;
        } else {
            fs::copy(entry.path(), b.join(entry.file_name()))
                .with_context(|| format!("failed to copy {:?}", entry.path()))?;
        }
    }
    Ok(())
}
