fn main() {
    // As part of the installation process from crates.io, try to move ourselves
    // to the `cargo-wasi` executable if we were a binary temporarily written
    // elsewhere to disk. Ignore most errors here though in case anything goes
    // wrong since this is largely just opportunistic.
    if let Some(dst) = std::env::var_os("__CARGO_WASI_RENAME_TO") {
        if let Ok(me) = std::env::current_exe() {
            drop(std::fs::rename(me, dst));
        }
    }

    println!("Hello, world!");
}
