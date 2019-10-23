use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

// Figure out where our bytes are coming from statically and include it as an
// optional list of bytes where `None` means "go fetch this from crates.io"
cfg_if::cfg_if! {
    if #[cfg(feature = "locally-developed")] {
        const BYTES: Option<&[u8]> = Some(include_bytes!(env!("BYTES_LOC")));
    } else if #[cfg(all(target_os = "windows", target_arch = "x86_64"))] {
        const BYTES: Option<&[u8]> = Some(cargo_wasi_exe_x86_64_pc_windows_msvc::BYTES);
    } else if #[cfg(all(target_os = "linux", target_arch = "x86_64"))] {
        const BYTES: Option<&[u8]> = Some(cargo_wasi_exe_x86_64_unknown_linux_musl::BYTES);
    } else if #[cfg(all(target_os = "macos", target_arch = "x86_64"))] {
        const BYTES: Option<&[u8]> = Some(cargo_wasi_exe_x86_64_apple_darwin::BYTES);
    } else {
        const BYTES: Option<&[u8]> = None;
    }
}

fn main() {
    let bytes = match BYTES {
        Some(n) => n,
        // FIXME(#1) - implement this
        None => panic!("unsupported fallback platform"),
    };

    // Figure out where our precompiled file will be written to disk. Currently
    // we use `.my-name` where `my-name` is the name of this executable
    let mut args = std::env::args_os();
    let me = match args.next() {
        Some(name) => name,
        None => std::process::exit(1),
    };
    let path = PathBuf::from(me);
    let file_name = match path.file_name().and_then(|f| f.to_str()) {
        Some(s) => format!(".{}", s),
        None => std::process::exit(2),
    };
    let candidate = path.with_file_name(file_name);

    // Create a `Command` which forwards all the arguments from this binary to
    // the next, and we'll be trying to execute it below.
    let mut cmd = Command::new(&candidate);
    for arg in args {
        cmd.arg(arg);
    }

    // On Unix we can rename the target executable on top of ourselves, but on
    // Windows this never works so don't even try.
    //
    // FIXME(#2) should fix this for Windows as well
    if cfg!(unix) {
        cmd.env("__CARGO_WASI_RENAME_TO", &path);
    }

    // Immediately try to execute this binary. If it doesn't exist then we need
    // to actually write it out to disk, but if it does exist then hey we saved
    // a few syscalls!
    match &exec(&mut cmd) {
        Ok(()) => return,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => {
            eprintln!("failed to spawn child process: {}", e);
            std::process::exit(3);
        }
    }

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
    if let Err(e) = opts.open(&candidate).and_then(|mut f| f.write_all(bytes)) {
        eprintln!(
            "failed to write executable file `{}`: {}",
            candidate.display(),
            e
        );
        std::process::exit(4);
    }

    // And finally run `exec` again, but this time we fail on all errors.
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
