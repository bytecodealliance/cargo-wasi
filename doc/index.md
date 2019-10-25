# Introduction

The `cargo-wasi` project is a subcommand for
[Cargo](https://doc.rust-lang.org/cargo/) which provides a convenient set of
defaults for building and running [Rust](https://doc.rust-lang.org/cargo/) code
on the [`wasm32-wasi` target](https://wasi.dev/). The `cargo wasi` command
makes compiling Rust code to WASI buttery-smooth with built-in defaults to avoid
needing to manage a myriad of tools as part of building a wasm executable.

[WASI is a developing standard](https://github.com/webassembly/wasi) and we hope
to make it very easy to develop Rust code for WASI to both influence the
standard as well as ensure that Rust code follows WASI best practices. Keep
reading for more information about how this all works!
