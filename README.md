# `cargo wasi` is deprecated, use [`cargo component`] instead

This repository for the `cargo wasi` tool was created long before the Component
Model of today in a time where the future of WASI was much less certain than it
is now. Nowadays users looking to integrate Rust and WASI should use [`cargo
component`] instead of `cargo wasi.

The original assumptions of `cargo wasi`, such as being based on `wasm-bindgen`,
are no longer applicable and the design direction of WASI has changed
significantly relative to when this tool was started.

See [this
comment](https://github.com/bytecodealliance/cargo-wasi/issues/143#issue-1839621636)
for a few more details. Otherwise feel free to reach out on [Zulip] with any
questions.

[`cargo component`]: https://github.com/bytecodealliance/cargo-component
[Zulip]: https://bytecodealliance.zulipchat.com/

---------

<div align="center">
  <h1><code>cargo wasi</code></h1>

<strong>A <a href="https://bytecodealliance.org/">Bytecode Alliance</a> project</strong>

  <p>
    <strong>A lightweight Cargo subcommand to build code for the <code>wasm32-wasi</code> target.</strong>
  </p>

  <p>
    <a href="https://crates.io/crates/cargo-wasi"><img src="https://img.shields.io/crates/v/cargo-wasi.svg?style=flat-square" alt="Crates.io version" /></a>
    <a href="https://crates.io/crates/cargo-wasi"><img src="https://img.shields.io/crates/d/cargo-wasi.svg?style=flat-square" alt="Download" /></a>
    <a href="https://bytecodealliance.github.io/cargo-wasi/"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs docs" /></a>
  </p>

  <h3>
    <a href="https://bytecodealliance.github.io/cargo-wasi/">Guide</a>
    <span> | </span>
    <a href="https://bytecodealliance.github.io/cargo-wasi/contributing.html">Contributing</a>
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

[**Read more about installation in the
guide!**](https://bytecodealliance.github.io/cargo-wasi/install.html)

## Usage

The `cargo wasi` subcommand is a thin wrapper around `cargo` subcommands,
providing optimized defaults for the `wasm32-wasi` target. Using `cargo wasi`
looks very similar to using `cargo`:

* `cargo wasi build` — build your code in debug mode for the wasi target.

* `cargo wasi build --release` — build the optimized version of your `*.wasm`.

* `cargo wasi run` — execute a binary.

* `cargo wasi test` — run your tests in `wasm32-wasi`.

* `cargo wasi bench` — run your benchmarks in `wasm32-wasi`.

In general, if you'd otherwise execute `cargo foo --flag` you can likely execute
`cargo wasi foo --flag` and everything will "just work" for the `wasm32-wasi`
target.

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

Or a library with some tests:

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

[**Read more about `cargo wasi` usage in the
guide!**](https://bytecodealliance.github.io/cargo-wasi/cli-usage.html)

## License

This project is license under the Apache 2.0 license with the LLVM exception.
See [LICENSE] for more details.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be licensed as above, without any additional terms or conditions.

[**See the contributing section of the guide to start hacking on `cargo
wasi`!**](https://bytecodealliance.github.io/cargo-wasi/contributing.html)
