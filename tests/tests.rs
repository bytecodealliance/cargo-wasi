use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

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
