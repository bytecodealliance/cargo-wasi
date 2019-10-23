fn main() {
    // If we're installed via a crates.io precompiled binary then we need to
    // clean up the self-installer binary. On Windows we must attempt this
    // every single time because on the first invocation we can't delete the
    // originally running binary. On Unix though this should always succeed on
    // the first try, so assert as such.
    //
    // Let's clarify windows. The first execution of this executable has the
    // crates.io shim installer as the parent process. On Unix that process
    // will be replaced via `exec` but on Windows it's a running process. In
    // this situation we won't be able to delete our sibling self-installer
    // from crates.io. On the second execution of this binary, though, the
    // self-installer won't be currently executing so we'll be able to delete
    // it. As a result we just always try deleting it and if it fails
    // we only consider that fatal on Unix.
    let try_self_delete =
        cfg!(windows) || std::env::var_os("__CARGO_WASI_SELF_DELETE_FOR_SURE").is_some();
    if try_self_delete {
        let me = match std::env::current_exe() {
            Ok(path) => path,
            Err(e) => {
                eprintln!("failed to get the current executable path: {}", e);
                std::process::exit(1);
            }
        };
        let filename = match me.file_name().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => std::process::exit(2),
        };
        let to_delete = me.with_file_name(format!(".{}", filename));
        if let Err(e) = std::fs::remove_file(to_delete) {
            if cfg!(unix) {
                eprintln!("failed to rename executable to final location: {}", e);
                std::process::exit(1);
            }
        }
    }

    cargo_wasi::main();
}
