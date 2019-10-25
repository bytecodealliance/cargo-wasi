# Hello, World!

Let's see an example of how to run the WASI version of "Hello, World!". This'll
end up looking very familiar to the Rust version of "Hello, World!" as well.
First up let's create a new project with Cargo:

```
$ cargo new wasi-hello-world
     Created binary (application) `wasi-hello-world` package
$ cd wasi-hello-world
```

This creates a `wasi-hello-world` folder which has a default `Cargo.toml` and
`src/main.rs`. The `main.rs` is the entry point for our program and currently
contains `println!("Hello, world!");`. Everything should be set up for us to
execute (no code needed!) so let's run the code inside of the `wasi-hello-world`
directory:

```
$ cargo wasi run
error: failed to find `wasmtime` in $PATH, you'll want to install `wasmtime` before running this command
...
```

Oh dear, that failed very quickly! For this command though we need to have a
way to actually execute the WebAssembly binary that Rust will produce. The
`cargo wasi` subcommand by default supports [wasmtime](https://wasmtime.dev),
and the error message should have instructions of how to install `wasmtime`.
You can also view installation instructions on the [wasmtime
website](https://wasmtime.dev).

Once we've got `wasmtime` installed, make sure it's working via:

```
$ wasmtime --version
```

Note that you may have to open a new shell for this to ensure `PATH` changes
take effect.

Ok, now that we've got a runtime installed, let's retry executing our binary:

```
$ cargo wasi run
info: downloading component 'rust-std' for 'wasm32-wasi'
info: installing component 'rust-std' for 'wasm32-wasi'
   Compiling wasi-hello-world v0.1.0 (/code/wasi-hello-world)
    Finished dev [unoptimized + debuginfo] target(s) in 0.15s
     Running `/.cargo/bin/cargo-wasi target/wasm32-wasi/debug/wasi-hello-world.wasm`
     Running `target/wasm32-wasi/debug/wasi-hello-world.wasm`
Hello, world!
```

Success! The command first used
[`rustup`](https://github.com/rust-lang/rustup.rs) to install the Rust
`wasm32-wasi` target automatically, and then we executed `cargo` to build the
WebAssembly binary. Finally `wasmtime` was used and we can see that `Hello,
world!` was printed by our program.

After this we're off to the races in developing our crate. Be sure to check out
the rest of this book for more information about what you can do with `cargo
wasi`. Additionally if this is your first time using Cargo, be sure to check
out [Cargo's introductory
documentation](https://doc.rust-lang.org/book/ch01-03-hello-cargo.html) as well
