use std::path::PathBuf;
use std::io::{self, Write};
use std::process::Command;
use std::fs::OpenOptions;

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
        None => panic!("unsupported fallback platform"),
    };
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

    let mut cmd = Command::new(&candidate);
    for arg in args {
        cmd.arg(arg);
    }
    match &exec(&mut cmd) {
        Ok(()) => return,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => {
            eprintln!("failed to spawn child process: {}", e);
            std::process::exit(3);
        }
    }

    let mut opts = OpenOptions::new();
    opts.create(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::prelude::*;
        opts.mode(0o755);
    }
    if let Err(e) = opts.open(&candidate).and_then(|mut f| f.write_all(bytes)) {
        eprintln!("failed to write executable file `{}`: {}", candidate.display(), e);
        std::process::exit(4);
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
