use anyhow::{Context, Result};
use assert_cmd::prelude::*;
use predicates::prelude::*;
use predicates::str::is_match;
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

#[test]
fn rust_names_demangled() -> Result<()> {
    let p = support::project()
        .file("src/main.rs", "fn main() {}")
        .build();

    p.cargo_wasi("build").assert().success();
    let bytes = std::fs::read(p.debug_wasm("foo")).context("failed to read wasm")?;
    assert_demangled(&bytes)?;

    p.cargo_wasi("build --release").assert().success();
    let bytes = std::fs::read(p.release_wasm("foo")).context("failed to read wasm")?;
    assert_demangled(&bytes)?;
    Ok(())
}

fn assert_demangled(wasm: &[u8]) -> Result<()> {
    let mut parser = wasmparser::ModuleReader::new(&wasm)?;
    let mut saw_name = false;
    while !parser.eof() {
        let section = parser.read()?;
        match section.code {
            wasmparser::SectionCode::Custom { name: "name", .. } => {}
            _ => continue,
        }
        saw_name = true;

        let mut reader = section.get_name_section_reader()?;
        while !reader.eof() {
            let functions = match reader.read()? {
                wasmparser::Name::Module(_) => continue,
                wasmparser::Name::Function(f) => f,
                wasmparser::Name::Local(_) => continue,
            };
            let mut map = functions.get_map()?;
            for _ in 0..map.get_count() {
                let name = map.read()?;
                if name.name.contains("ZN") {
                    panic!("still-mangled name {:?}", name.name);
                }
            }
        }
    }
    assert!(saw_name);
    Ok(())
}

#[test]
fn check_output() -> Result<()> {
    // download the wasi target and get that out of the way
    support::project()
        .file("src/main.rs", "fn main() {}")
        .build()
        .cargo_wasi("check")
        .assert()
        .success();

    // Default output
    support::project()
        .file("src/main.rs", "fn main() {}")
        .build()
        .cargo_wasi("build")
        .assert()
        .stdout("")
        .stderr(is_match(
            "^\
.*Compiling foo v1.0.0 .*
.*Finished dev .*
$",
        )?)
        .success();

    let p = support::project()
        .file("src/main.rs", "fn main() {}")
        .build();

    // Default verbose output
    p.cargo_wasi("build -v")
        .assert()
        .stdout("")
        .stderr(is_match(
            "^\
.*Running \"cargo\" .*
.*Compiling foo v1.0.0 .*
.*Running `rustc.*`
.*Finished dev .*
.*Processing .*foo.rustc.wasm
$",
        )?)
        .success();

    // Incremental verbose output
    p.cargo_wasi("build -v")
        .assert()
        .stdout("")
        .stderr(is_match(
            "^\
.*Running \"cargo\" .*
.*Fresh foo v1.0.0 .*
.*Finished dev .*
$",
        )?)
        .success();

    // Incremental non-verbose output
    p.cargo_wasi("build")
        .assert()
        .stdout("")
        .stderr(is_match(
            "^\
.*Finished dev .*
$",
        )?)
        .success();

    Ok(())
}

#[test]
fn check_output_release() -> Result<()> {
    // download the wasi target and get that out of the way
    support::project()
        .file("src/main.rs", "fn main() {}")
        .build()
        .cargo_wasi("build --release")
        .assert()
        .success();

    // Default output
    support::project()
        .file("src/main.rs", "fn main() {}")
        .build()
        .cargo_wasi("build --release")
        .assert()
        .stdout("")
        .stderr(is_match(
            "^\
.*Compiling foo v1.0.0 .*
.*Finished release .*
.*Optimizing with wasm-opt
$",
        )?)
        .success();

    let p = support::project()
        .file("src/main.rs", "fn main() {}")
        .build();

    // Default verbose output
    p.cargo_wasi("build -v --release")
        .assert()
        .stdout("")
        .stderr(is_match(
            "^\
.*Running \"cargo\" .*
.*Compiling foo v1.0.0 .*
.*Running `rustc.*`
.*Finished release .*
.*Processing .*foo.rustc.wasm
.*Optimizing with wasm-opt
.*Running \".*wasm-opt\" .*
$",
        )?)
        .success();

    // Incremental verbose output
    p.cargo_wasi("build -v --release")
        .assert()
        .stdout("")
        .stderr(is_match(
            "^\
.*Running \"cargo\" .*
.*Fresh foo v1.0.0 .*
.*Finished release .*
$",
        )?)
        .success();

    // Incremental non-verbose output
    p.cargo_wasi("build --release")
        .assert()
        .stdout("")
        .stderr(is_match(
            "^\
.*Finished release .*
$",
        )?)
        .success();

    Ok(())
}

// feign the actual `wasm-bindgen` here because it takes too long to compile
#[test]
fn wasm_bindgen() -> Result<()> {
    let p = support::project()
        .file(
            "Cargo.toml",
            r#"
                [package]
                name = "foo"
                version = '1.0.0'

                [dependencies]
                wasm-bindgen = { path = 'wasm-bindgen' }
            "#,
        )
        .file("src/main.rs", "fn main() {}")
        .file(
            "wasm-bindgen/Cargo.toml",
            r#"
                [package]
                name = "wasm-bindgen"
                version = '1.0.0'
            "#,
        )
        .file("wasm-bindgen/src/lib.rs", "")
        .build();

    p.cargo_wasi("build -v")
        .env("WASM_BINDGEN", "my-wasm-bindgen")
        .assert()
        .stdout("")
        .stderr(is_match(
            "^\
.*Running \"cargo\" .*
.*Compiling wasm-bindgen v1.0.0 .*
.*Running `rustc.*`
.*Compiling foo v1.0.0 .*
.*Running `rustc.*`
.*Finished dev .*
error: failed to process wasm at `.*foo.rustc.wasm`

Caused by:
    failed to create process \"my-wasm-bindgen\".*\"--keep-debug\".*

Caused by:
    .*
$",
        )?)
        .code(1);

    p.cargo_wasi("build")
        .env("WASM_BINDGEN", "my-wasm-bindgen")
        .assert()
        .stdout("")
        .stderr(is_match(
            "^\
.*Finished dev .*
error: failed to process wasm at `.*foo.rustc.wasm`

Caused by:
    failed to create process \"my-wasm-bindgen\".*

Caused by:
    .*
$",
        )?)
        .code(1);

    p.cargo_wasi("build --release")
        .env("WASM_BINDGEN", "my-wasm-bindgen")
        .assert()
        .stdout("")
        .stderr(is_match(
            "^\
.*Compiling wasm-bindgen .*
.*Compiling foo .*
.*Finished release .*
error: failed to process wasm at `.*foo.rustc.wasm`

Caused by:
    failed to create process \"my-wasm-bindgen\".*

Caused by:
    .*
$",
        )?)
        .code(1);

    Ok(())
}

#[test]
fn run() -> Result<()> {
    support::project()
        .file("src/main.rs", "fn main() {}")
        .build()
        .cargo_wasi("run")
        .assert()
        .stdout("")
        .stderr(is_match(
            "^\
.*Compiling foo v1.0.0 .*
.*Finished dev .*
.*Running `.*`
.*Running `.*`
$",
        )?)
        .success();

    support::project()
        .file(
            "src/main.rs",
            r#"
                fn main() { println!("hello") }
            "#,
        )
        .build()
        .cargo_wasi("run")
        .assert()
        .stdout("hello\n")
        .stderr(is_match(
            "^\
.*Compiling foo v1.0.0 .*
.*Finished dev .*
.*Running `.*`
.*Running `.*`
$",
        )?)
        .success();
    Ok(())
}

#[test]
fn run_forward_args() -> Result<()> {
    support::project()
        .file(
            "src/main.rs",
            r#"
                fn main() {
                    println!("{:?}", std::env::args().skip(1).collect::<Vec<_>>());
                }
            "#,
        )
        .build()
        .cargo_wasi("run a -- -b c")
        .assert()
        .stdout("[\"a\", \"-b\", \"c\"]\n")
        .success();
    Ok(())
}

#[test]
fn test() -> Result<()> {
    support::project()
        .file(
            "src/lib.rs",
            r#"
                #[test]
                fn smoke() {}
            "#,
        )
        .build()
        .cargo_wasi("test")
        .assert()
        .stdout(
            "
running 1 test
test smoke ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

",
        )
        .stderr(is_match(
            "^\
.*Compiling foo v1.0.0 .*
.*Finished dev .*
.*Running .*
.*Running `.*`
$",
        )?)
        .success();
    Ok(())
}

#[test]
fn run_nothing() -> Result<()> {
    support::project()
        .file("src/lib.rs", "")
        .build()
        .cargo_wasi("run")
        .assert()
        .code(1);
    Ok(())
}

#[test]
fn run_many() -> Result<()> {
    support::project()
        .file("src/bin/foo.rs", "")
        .file("src/bin/bar.rs", "")
        .build()
        .cargo_wasi("run")
        .assert()
        .code(1);
    Ok(())
}

#[test]
fn run_one() -> Result<()> {
    support::project()
        .file("src/bin/foo.rs", "fn main() {}")
        .file("src/bin/bar.rs", "")
        .build()
        .cargo_wasi("run --bin foo")
        .assert()
        .code(0);
    Ok(())
}
