use anyhow::Context;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> anyhow::Result<()> {
    let tmp = Path::new("tmp");
    drop(fs::remove_dir_all(&tmp));
    fs::create_dir_all(tmp.join("shim")).context("failed to create `tmp` dir")?;

    println!("Copying the shim into `tmp`...");
    cp_r("crates/cargo-wasi-shim".as_ref(), &tmp.join("shim"))?;

    let mut overrides = Vec::new();
    for dir in std::env::args().skip(1) {
        let dir = Path::new(&dir);
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
        overrides.push(krate.file_stem().unwrap().to_str().unwrap().to_string());
    }

    println!("Rewriting shim manifest with `[patch]`");
    let manifest_path = tmp.join("shim/Cargo.toml");
    let mut manifest = fs::read_to_string(&manifest_path)
        .context("failed to read manifest")?
        .replace("cargo-wasi-shim", "cargo-wasi");
    manifest.push_str("\n");
    manifest.push_str("[patch.crates-io]\n");
    for name in overrides.iter() {
        let krate = &name[..name.as_bytes().iter().rposition(|b| *b == b'-').unwrap()];
        manifest.push_str(&format!("{} = {{ path = '../{}' }}\n", krate, name));
    }
    fs::write(&manifest_path, manifest).context("failed to write manifest")?;

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

    let mut entries = tmp
        .read_dir()?
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();
    entries.sort();
    for entry in entries {
        println!("publish {:?}", entry);
        let mut cmd = Command::new("cargo");
        cmd.arg("publish").current_dir(&entry);
        if std::env::var("NO_DRY_RUN").is_err() {
            cmd.arg("--dry-run");

            if entry.ends_with("tmp/shim") {
                println!("skipping `shim` since crates aren't published");
                continue;
            }
        } else {
            // give crates.io a chance to propagate the index change
            std::thread::sleep(std::time::Duration::from_secs(5));
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
