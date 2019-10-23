use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

// Figure out where our bytes are coming from statically and include it as an
// optional list of bytes where `None` means "run `cargo_wasi_main`"
//
// Note that this block really only works with the published version of this
// crate which is found on crates.io. That means the manifest has been
// transformed by `weave.rs` and notably has a bunch of injected precompiled
// dependencies. If we do not have a precompiled dependency then this crate will
// fall back to depending on `cargo-wasi-src` which is the actual `cargo-wasi`
// crate itself, so our fallback simply runs the main function there.
//
// This is sort of a "glorious hack" to severely reduce the compile times of
// this crate (and compile requirements) if you're installing on a platform that
// we have precompiled binaries for. Only if you're installing for a platform
// that doesn't have precompiled binaries do things go awry.
cfg_if::cfg_if! {
    if #[cfg(feature = "locally-developed")] {
        const BYTES: Option<&[u8]> = Some(include_bytes!(env!("BYTES_LOC")));
        fn cargo_wasi_main() { unreachable!() }
    } else if #[cfg(all(target_os = "windows", target_arch = "x86_64"))] {
        const BYTES: Option<&[u8]> = Some(cargo_wasi_exe_x86_64_pc_windows_msvc::BYTES);
        fn cargo_wasi_main() { unreachable!() }
    } else if #[cfg(all(target_os = "linux", target_arch = "x86_64"))] {
        const BYTES: Option<&[u8]> = Some(cargo_wasi_exe_x86_64_unknown_linux_musl::BYTES);
        fn cargo_wasi_main() { unreachable!() }
    } else if #[cfg(all(target_os = "macos", target_arch = "x86_64"))] {
        const BYTES: Option<&[u8]> = Some(cargo_wasi_exe_x86_64_apple_darwin::BYTES);
        fn cargo_wasi_main() { unreachable!() }
    } else {
        const BYTES: Option<&[u8]> = None;
        fn cargo_wasi_main() { cargo_wasi_src::main() }
    }
}

fn main() {
    // If we have a precompiled binary, run that, otherwise delegate to
    // `cargo_wasi_src`
    match BYTES {
        Some(n) => run_precompiled(n),
        None => cargo_wasi_main(),
    }
}

/// Run a precompiled binary whose data is `bytes`.
///
/// This function will execute the precompiled binary `bytes` by writing it to
/// disk and then executing it. We want to write `bytes` as the current
/// executable but this unfortunately isn't an easy operation. The most
/// cross-platform and robust way of doing this is:
///
/// * Write the `bytes` to disk
/// * Swap the current executable and this temporary file
/// * Execute the temporary file, now named as this current executable
/// * Request that the new executable delete our own, perhaps after a few
///   executions in the future. On Windows on the initial execution it won't be
///   able to delete the source executable, but on Unix it should always be able
///   to do so.
fn run_precompiled(bytes: &[u8]) {
    // Figure out where our precompiled file will be written to disk. Currently
    // we use `.my-name` where `my-name` is the name of this executable
    let mut args = std::env::args_os();
    let me = match args.next() {
        Some(name) => name,
        None => std::process::exit(1),
    };
    let path = PathBuf::from(me.clone());
    let file_name = match path.file_name().and_then(|f| f.to_str()) {
        Some(s) => format!(".{}", s),
        None => std::process::exit(2),
    };
    let temporary = path.with_file_name(".cargo-wasi-tmp");
    let our_destination = path.with_file_name(file_name);

    // Write out an executable file to disk containing `bytes` at our determined
    // location. Note that on Unix we need to set the file's mode, but on
    // Windows everything is inherently executable.
    let mut opts = OpenOptions::new();
    opts.create(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::prelude::*;
        opts.mode(0o755);
    }
    if let Err(e) = opts.open(&temporary).and_then(|mut f| f.write_all(bytes)) {
        eprintln!(
            "failed to write executable file `{}`: {}",
            temporary.display(),
            e
        );
        std::process::exit(3);
    }

    if let Err(e) = fs::rename(&me, &our_destination) {
        eprintln!(
            "failed to move this binary to `{}`: {}",
            our_destination.display(),
            e
        );
        std::process::exit(4);
    }
    if let Err(e) = fs::rename(&temporary, &me) {
        // oh dear we are very messed up if this fails. Our executable name
        // (cargo-wasi) no longer exists, so do a last-ditch effort to try to
        // put ourselves back.
        drop(fs::rename(&our_destination, &me));
        eprintln!("failed to create binary at {:?}: {}", me, e);
        std::process::exit(5);
    }

    // Now re-exec `&me` since it's a different binary. If we're on unix then
    // we definitely should be able to delete our shim file.
    let mut cmd = Command::new(&me);
    for arg in args {
        cmd.arg(arg);
    }
    if cfg!(unix) {
        cmd.env("__CARGO_WASI_SELF_DELETE_FOR_SURE", "1");
    }
    match exec(&mut cmd) {
        Ok(()) => return,
        Err(e) => {
            eprintln!("failed to spawn child process: {}", e);
            std::process::exit(5);
        }
    }
}

#[cfg(unix)]
fn exec(cmd: &mut Command) -> io::Result<()> {
    use std::os::unix::prelude::*;
    Err(cmd.exec())
}

#[cfg(windows)]
fn exec(cmd: &mut Command) -> io::Result<()> {
    let status = cmd.status()?;
    std::process::exit(status.code().unwrap_or(6));
}
