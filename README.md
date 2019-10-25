<div align="center">
  <h1><code>cargo wasi</code></h1>

  <p>
    <strong>A lightweight Cargo subcommand to build code for the `wasm32-wasi` target.</strong>
  </p>

  <p>
    <a href="https://crates.io/crates/cargo-wasi"><img src="https://img.shields.io/crates/v/cargo-wasi.svg?style=flat-square" alt="Crates.io version" /></a>
  </p>

  <h3>
    <a href="https://alexcrichton.github.io/cargo-wasi/">Guide</a>
    <span> | </span>
    <a href="https://alexcrichton.github.io/cargo-wasi/contributing.html">Contributing</a>
  </h3>
</div>

## Installation

To install this Cargo subcommand, first you'll want to [install
Rust](https://www.rust-lang.org/tools/install) and then you'll execute:

```
$ cargo install cargo-wasi
```

After that you can verify it works via:

```
$ cargo wasi --version
```

[Read more about
installation](https://alexcrichton.github.io/cargo-wasi/install.html)

## Usage

The `cargo wasi` subcommand is a thin wrapper around `cargo` subcommands,
providing optimized defaults for the `wasm32-wasi` target. Because of this usage
of `cargo wasi` looks very similar to Cargo itself:

* `cargo wasi build` - build your code in debug mode for the wasi target.
* `cargo wasi build --release` - build the optimized version of your `*.wasm`.
* `cargo wasi run` - execute a binary
* `cargo wasi test` - run your tests in `wasm32-wasi`
* `cargo wasi bench` - run your benchmarks in `wasm32-wasi`

And that's just a taste! In general if you'd otherwise execute `cargo foo
--flag` you can likely execute `cargo wasi foo --flag` and everything will "just
work" for the wasi target. For more long-form documentation, examples, and more
explanation, be sure to consult the [book
documentation](https://alexcrichton.github.io/cargo-wasi) for this subcommand as
well.

To give it a spin yourself, try out the hello-world versions of programs!

```
$ cargo new wasi-hello-world
     Created binary (application) `wasi-hello-world` package
$ cd wasi-hello-world
$ cargo wasi run
   Compiling wasi-hello-world v0.1.0 (/code/wasi-hello-world)
    Finished dev [unoptimized + debuginfo] target(s) in 0.15s
     Running `cargo-wasi target/wasm32-wasi/debug/wasi-hello-world.wasm`
     Running `target/wasm32-wasi/debug/wasi-hello-world.wasm`
Hello, world!
```

or a library with some tests:

```
$ cargo new wasi-hello-world --lib
     Created library `wasi-hello-world` package
$ cd wasi-hello-world
$ cargo wasi test
   Compiling wasi-hello-world v0.1.0 (/code/wasi-hello-world)
    Finished dev [unoptimized + debuginfo] target(s) in 0.19s
     Running target/wasm32-wasi/debug/deps/wasi_hello_world-9aa88657c21196a1.wasm
     Running `/code/wasi-hello-world/target/wasm32-wasi/debug/deps/wasi_hello_world-9aa88657c21196a1.wasm`

running 1 test
test tests::it_works ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

## Configuration

The `cargo wasi` subcommand takes no flags itself since it forwards all flags to
Cargo itself. To configure `cargo wasi` you'll be editing your workspace
`Cargo.toml`:

```toml
[package.metadata]
wasm-opt = false              # force wasm-opt to never run
wasm-name-section = false     # remove the name section from release artifacts
# ...
```

For a full list of configuration options see the
[documentation](https://alexcrichton.github.io/cargo-wasi/config.html).

## Updating `cargo-wasi`

If you already have `cargo-wasi` installed and you'd like to update your
installation, you can execute:

```
$ cargo install cargo-wasi --force
```

## Uninstalling `cargo-wasi`

If you'd like to remove `cargo-wasi` from your system, you'll want to first
clear out the subcommand's caches and then remove the subcommand itself.

```
$ cargo wasi self clean
$ cargo uninstall cargo-wasi
```

# License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
