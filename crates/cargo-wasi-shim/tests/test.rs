use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn me() -> PathBuf {
    let mut me = std::env::current_exe().unwrap();
    me.pop();
    me.pop();
    me.push("cargo-wasi-shim");
    me.set_extension(std::env::consts::EXE_EXTENSION);
    return me;
}

fn case() -> (tempfile::TempDir, PathBuf) {
    let td = tempfile::TempDir::new().unwrap();
    let path = td.path().join("cargo-wasi");
    fs::copy(me(), &path).unwrap();
    (td, path)
}

#[test]
fn smoke() {
    let (_td, path) = case();
    let before = fs::read(&path).unwrap();
    let output = Command::new(&path).output().unwrap();
    println!("{:#?}", output);
    assert!(output.status.success());
    let after = fs::read(&path).unwrap();
    if after == before {
        assert!(path.with_file_name(".cargo-wasi").exists());
        assert!(cfg!(windows));
    } else {
        assert!(!path.with_file_name(".cargo-wasi").exists());
        assert!(!cfg!(windows));
    }
}

#[test]
fn pass_args() {
    let (_td, path) = case();
    let output = Command::new(&path).arg("--help").output().unwrap();
    println!("{:#?}", output);
    assert!(output.status.success());
}

#[test]
fn run_twice() {
    let (_td, path) = case();
    let output = Command::new(&path).output().unwrap();
    println!("{:#?}", output);
    assert!(output.status.success());
    let output = Command::new(&path).output().unwrap();
    println!("{:#?}", output);
    assert!(output.status.success());
}
