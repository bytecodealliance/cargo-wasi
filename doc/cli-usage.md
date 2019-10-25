# CLI Usage

In general `cargo wasi` takes no CLI flags specifically, since it will forward
*everything* to `cargo` under the hood. The subcommand, however, will attempt
to infer flags such as `-v` from the Cargo arguments pass, switching itself to
a verbose output if it looks like Cargo is using a verbose output.

The supported subcommands for `cargo wasi` are:

## `cargo wasi build`

This is the primary subcommand used to build WebAssembly code. This will build
your crate for the `wasm32-wasi` target and run any postprocessing (like
`wasm-bindgen` or `wasm-opt`) over any produced binary.

```
$ cargo wasi build
$ cargo wasi build --release
$ cargo wasi build --lib
$ cargo wasi build --test foo
```

Output `*.wasm` files will be located in `target/wasm32-wasi/debug` for debug
builds or `target/wasm32-wasi/release` for release builds.

## `cargo wasi check`

This subcommands forwards everything to `cargo check`, allowing to perform
quick compile-time checks over your code without actually producing any
`*.wasm` binaries or running any wasm code.

```
$ cargo wasi check
$ cargo wasi check --lib
$ cargo wasi check --tests
```

## `cargo wasi run`

Forwards everything to `cargo run`, and runs all binaries in `wasmtime`.
Arguments passed will be forwarded to `cargo bench`. Note that it's not
necessary to run `cargo wasi build` before this subcommand. Example usage looks
like:

```
$ cargo wasi run
$ cargo wasi run --release
$ cargo wasi run arg1 arg2
$ cargo wasi run -- --flag-for-wasm-binary
$ cargo wasi run --bin foo
```

> **Note**: Using `cargo wasi` will print `Running ...` twice, that's normal
> but only one wasm binary is actually run.

## `cargo wasi test`

Forwards everything to `cargo test`, and runs all tests in `wasmtime`.
Arguments passed will be forwarded to `cargo test`. Note that it's not
necessary to run `cargo wasi build` before executing this command. Example
usage looks like:

```
$ cargo wasi test
$ cargo wasi test my_test_to_run
$ cargo wasi test --lib
$ cargo wasi test --test foo
$ cargo wasi test -- --nocpature
```

You can find some more info about writing tests in the [Rust book's chapter on
writing tests](https://doc.rust-lang.org/book/ch11-01-writing-tests.html).

> **Note**: You'll also want to be sure to consult [WASI-specific caveats when
testing](testing.md) since there are some gotchas today.

## `cargo wasi bench`

Forwards everything to `cargo bench`, and like previous commands also executes
the benchmarks inside of `wasmtime`. Arguments passed will be forwarded to
`cargo bench`, such as:

```
$ cargo wasi bench
$ cargo wasi bench my_benchmark_to_run
$ cargo wasi bench --bench foo
$ cargo wasi bench -- --nocpature
```

## `cargo wasi fix`

Forwards everything to `cargo fix`, but again with the `--target wasm32-wasi`
option which ensures that the fixes are also applied to wasi-specific code (if
any).

## `cargo wasi version`

This subcommand will print out version information about `cargo wasi` itself.
This is also known as `cargo wasi -V` and `cargo wasi --version`.

```
$ cargo wasi version
$ cargo wasi -V
$ cargo wasi --version
```

## `cargo wasi self clean`

This is an internal management subcommand for `cargo wasi` which completely
clears out the cache that `cargo wasi` uses for itself. This cache includes
various metadata files and downloaded versions of tools like `wasm-opt` and
`wasm-bindgen`.

```
$ cargo wasi self clean
```
