use anyhow::{Context, Result};
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

mod support;

fn cargo_wasi(args: &str) -> Command {
    let mut me = std::env::current_exe().unwrap();
    me.pop();
    me.pop();
    me.push("cargo-wasi");
    me.set_extension(std::env::consts::EXE_EXTENSION);

    let mut cmd = Command::new(&me);
    cmd.arg("wasi");
    for arg in args.split_whitespace() {
        cmd.arg(arg);
    }

    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut path = std::env::split_paths(&path).collect::<Vec<_>>();
    path.insert(0, me);
    cmd.env("PATH", std::env::join_paths(&path).unwrap());

    return cmd;
}

#[test]
fn help() {
    cargo_wasi("help").assert().success();
}

#[test]
fn version() {
    cargo_wasi("-V")
        .assert()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")))
        .success();
    cargo_wasi("--version")
        .assert()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")))
        .success();
    cargo_wasi("version")
        .assert()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")))
        .success();
}

#[test]
fn contains_debuginfo() -> Result<()> {
    let p = support::project()
        .file("src/main.rs", "fn main() {}")
        .build();

    p.cargo_wasi("build").assert().success();
    let bytes = std::fs::read(p.debug_wasm("foo")).context("failed to read wasm")?;

    let mut parser = wasmparser::ModuleReader::new(&bytes)?;
    let mut any = false;
    while !parser.eof() {
        match parser.read()?.code {
            wasmparser::SectionCode::Custom { name, .. } => {
                if name.starts_with(".debug") {
                    any = true;
                }
            }
            _ => {}
        }
    }
    if !any {
        panic!("failed to find debuginfo");
    }
    Ok(())
}

#[test]
fn strip_debuginfo() -> Result<()> {
    let p = support::project()
        .file("src/main.rs", "fn main() {}")
        .build();

    p.cargo_wasi("build --release").assert().success();
    let bytes = std::fs::read(p.release_wasm("foo")).context("failed to read wasm")?;

    let mut parser = wasmparser::ModuleReader::new(&bytes)?;
    while !parser.eof() {
        match parser.read()?.code {
            wasmparser::SectionCode::Custom { name, .. } => {
                if name.starts_with(".debug") {
                    panic!("found `{}` section in wasm file", name);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[test]
fn check_works() {
    let p = support::project()
        .file("src/main.rs", "fn main() {}")
        .build();

    p.cargo_wasi("check").assert().success();
}

#[test]
fn fix_works() {
    let p = support::project()
        .file("src/main.rs", "fn main() {}")
        .build();

    p.cargo_wasi("fix --allow-no-vcs").assert().success();
}
