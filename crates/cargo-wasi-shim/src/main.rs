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
    match BYTES {
        Some(n) => println!("got {} bytes", n.len()),
        None => println!("no bytes"),
    }
}
