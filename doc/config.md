# Configuration

The `cargo wasi` subcomand [does not have any CLI flags of its
own](cli-usage.md) but it's still not a one-size-fits-all command, so
configuration needs to go somewhere! The `cargo wasi` command supports
[TOML](https://github.com/toml-lang/toml)-based configuration stored in your
[workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)
`Cargo.toml` in the `[package.metadata]` section:

```toml
[package.metadata]
# ...
```

The keys supported by `cargo wasi` are:

```toml
[package.metadata]
wasm-opt = true
wasm-name-section = true
wasm-producers-section = true
```

For more documentation about each key, see its section below.

## `wasm-opt`

This configuration option is a boolean value (`true` or `false`) which
indicates whether the `wasm-opt` optimization tool from the [binaryen
toolkit](https://github.com/webassembly/binaryen) will be executed to further
optimize the produced WebAssembly binaries. The default for this option is
`true`.

If this option is set to `false`, then `wasm-opt` will never be executed. If
this option is set to `true`, the `wasm-opt` will still not be run if debuginfo
is present or if `wasm-bindgen` is present. For more information about this see
[the documentation about running `wasm-opt`](wasm-opt.md).

## `wasm-name-section`

The [`name` custom
section](http://webassembly.github.io/spec/core/appendix/custom.html#name-section)
records debugging information as names for wasm functions and variables. If you
want reasonable stack traces or debug information it's recommended to have the
`name` section present. Builds optimized for size though that have other
channels of debugging may wish to disable this.

This configuration option is a boolean value (`true` or `false`) which
indicates whether the `name` section should be present or not. This option
defaults to `true`.

If this option is set to `false` then it only takes effect when a build is
produced without debuginfo. For example a `cargo wasi build` binary which has
debuginfo would still have the `name` section present. A `cargo wasi build
--release` binary, however, would not have debuginfo and would also have the
`name` section removed.

## `wasm-producers-section`

The [`producers` custom
section](https://github.com/WebAssembly/tool-conventions/blob/master/ProducersSection.md)
records tools used to produce a WebAssembly module. This is meant for metric
collection in production systems, and is generally harmless to include. Builds
micro-optimized for size, however, may wish to exclude it.

This configuration option is a boolean value (`true` or `false`) which
indicates whether the `producers` section should be present or not. This option
defaults to `true`.

If this option is set to `false` then it only takes effect when a build is
produced without debuginfo. For example a `cargo wasi build` binary which has
debuginfo would still have the `producers` section present. A `cargo wasi build
--release` binary, however, would not have debuginfo and would also have the
`producers` section removed.
