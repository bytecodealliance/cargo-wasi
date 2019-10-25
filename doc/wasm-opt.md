# Running `wasm-opt`

By default `cargo wasi` will run `wasm-opt` over optimized WebAssembly
binaries. The `wasm-opt` program is a tool in the [binaryen
toolkit](https://github.com/webassembly/binaryen) which is a wasm-to-wasm
transformation that optimizes the input wasm module. Often `wasm-opt` can get
10-20% size reductions over LLVM's raw output.

There are a number of heuristics that are used to configure how `wasm-opt` is
run though and it's important to keep those in mind!

## Which `wasm-opt` executed?

Every release of `cargo wasi` is hardcoded to download a precompiled version of
`wasm-opt`. This binary will be lazily downloaded and then executed. You can
also request that a specific `wasm-opt` binary is used via the `WASM_OPT`
environment variable.

Note that we're interested in feedback on this strategy, so please don't
hesitate to file an issue if this doesn't work for you!

## Disabled with DWARF debuginfo

If DWARF debug information is requested for a build (default on for debug
builds, default off for release builds) then `wasm-opt` will be disabled.
At the time of this writing `wasm-opt` does not support preserving DWARF debug
information through its transformations, so `wasm-opt` is skipped.

In effect this means that `wasm-opt` will not run in debug mode, but it will
run in release mode. If you enable debug info in release mode, though, then it
will not run.

You can configure debuginfo through your `Cargo.toml`:

```toml
[profile.release]
debug = 1
```

## Selected Optimization Level

The `wasm-opt` tool, like most compilers, supports multiple levels of
optimization. The optimization level is by default selected to match `rustc`'s
own optimization level. If `rustc`'s optimization level is "0", then `wasm-opt`
will not be run.

This effectively means that in debug mode this is another reason that
`wasm-opt` is disabled (because debug mode uses optimization level 0). In
release mode we will by default execute `wasm-opt -O3` because `rustc` is
executed with `-C opt-level=3`.

You can configure `rustc`'s and `wasm-opt`'s optimization level through your
`Cargo.toml`.  For example to optimize for size instead of speed:

```toml
[profile.release]
opt-level = 's'
```

## Disabled via configuration

You can also outright disable `wasm-opt` via [configuration](config.md) by
updating your `Cargo.toml`:

```toml
[package.metadata]
wasm-opt = false
```

## Disabled when `wasm-bindgen` is used

Finally, as one last caveat, `wasm-opt` is automatically disabled if
`wasm-bindgen` is used as part of the build. If `wasm-bindgen` is used it's
assumed that WebAssembly Interface Types are also used, and currently
`wasm-opt` (at the time of this writing) does not have support for WebAssembly
Interface Types. If we were to run `wasm-opt` it would produce a broken binary!
