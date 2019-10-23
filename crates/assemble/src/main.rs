use anyhow::Context;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> anyhow::Result<()> {
    let target = std::env::args().nth(1).unwrap();
    let binary = std::env::args().nth(2).unwrap();
    drop(fs::remove_dir_all("tmp"));
    fs::create_dir_all("tmp").context("failed to create `tmp`")?;

    let toml = fs::read_to_string("Cargo.toml").context("failed to read `Cargo.toml`")?;
    let toml: toml::Value = toml::from_str(&toml)?;
    let name = format!("cargo-wasi-exe-{}", target);

    fs::write(
        "tmp/Cargo.toml",
        format!(
            r#"
[package]
name = "{name}"
version = {version}
authors = {authors}
license = {license}
repository = {repository}
description = """
Precompiled binary of `cargo-wasi` for {target}
"""

[workspace]
            "#,
            name = name,
            target = target,
            version = toml["package"]["version"],
            authors = toml["package"]["authors"],
            repository = toml["package"]["repository"],
            license = toml["package"]["license"],
        ),
    )?;

    fs::create_dir("tmp/src").context("failed to create `src`")?;
    fs::write(
        "tmp/src/lib.rs",
        "pub const BYTES: &[u8] = include_bytes!(\"binary\");",
    )
    .context("failed to write `lib.rs`")?;
    fs::copy(&binary, "tmp/src/binary").context("failed to write `binary`")?;

    let status = Command::new("cargo")
        .arg("package")
        .current_dir("tmp")
        .env("CARGO_TARGET_DIR", "../target")
        .status()
        .context("failed to spawn `cargo`")?;
    if !status.success() {
        anyhow::bail!("packaging via `cargo` failed: {}", status);
    }

    fs::remove_dir_all("tmp").context("failed to remove `tmp`")?;
    fs::create_dir("tmp").context("failed to create `tmp`")?;

    let binary = Path::new(&binary);
    fs::copy(&binary, Path::new("tmp").join(binary.file_name().unwrap()))?;

    let filename = format!(
        "{}-{}.crate",
        name,
        toml["package"]["version"].as_str().unwrap()
    );
    let krate = Path::new("target/package").join(&filename);
    fs::copy(&krate, Path::new("tmp").join(&filename))
        .context(format!("failed to copy `{}`", krate.display()))?;

    Ok(())
}
