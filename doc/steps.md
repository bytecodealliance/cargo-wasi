# Steps run by `cargo wasi`

The `cargo wasi` subcommand is intended to be a *convenience* when developing
Rust code for WASI, but is not required. It is a thin wrapper around the general
"toolchain" of building WebAssembly code. Building WebAssembly code can be
relatively involved and have a nontrivial number of moving parts, so having a
convenience like `cargo wasi` becomes quite nice quite quickly, but it's
important to also understand what `cargo wasi` is doing under the hood!

This section will explain the various steps that `cargo wasi` internally takes
care of for you. Be sure to check out the [reference
documentation](reference.md) for an exhaustive list of ways to run and configure
`cargo wasi`.

## Managing the `wasm32-wasi` target

The Rust installer does not install the `wasm32-wasi` Rust standard library by
default, but to compile any code for `wasm32-wasi` you'll need to be sure to
have this target installed for your Rust toolchain. The `cargo wasi` subcommand
will automatically execute, if necessary:

```
rustup target add wasm32-wasi
```

For systems not using `rustup` it will generate an error indicating whether or
not the `wasm32-wasi` target is installed.

## Ensuring a `wasmtime` runtime is installed

As we saw previously when [running "Hello, world!"](hello-world.md) a
`wasmtime` executable is required to execute WASI code locally. The `cargo wasi`
subcommand will verify that it is installed and provide an understandable error
message if it isn't, also recommending how to [install
`wasmtime`](https://wasmtime.dev).

## Automatically configure Cargo for `wasm32-wasi`

Whenever `cargo wasi` is used it will automatically pass `--target wasm32-wasi`
to all Cargo subcommands that are invoked. This avoids you having to type
this all out on each command.

## Further optimizing WebAssembly with `wasm-opt`

The Rust compiler usese LLVM's WebAssembly backend to produce WebAssembly code.
LLVM itself is an extremely good optimizing compiler, but LLVM's WebAssembly
backend is unfortunately not quite as optimized as its other backends (such as
X86). Standard practice today is to execute the `wasm-opt` tool (part of the
[binaryen project](https://github.com/webassembly/binaryen)) to further
optimize a WebAssembly binary.

For LLVM-optimized WebAssembly binaries `wasm-opt` normally doesn't get much of
a runtime speed increase, but it can often reduce the size of a WebAssembly
binary by 10-20%, which can be some serious savings!

For more information about how `wasm-opt` is run see the [reference
documentation](wasm-opt.md)

## Executing `wasm-bindgen` for WebAssembly Interface Types

The [WebAssembly Interface Types
proposal](https://github.com/webassembly/interface-types) is a developing
standard for enhancing the set of types that a WebAssembly module can work with
at its boundaries (as opposed to just integers and floats). This developing
standard is targeted at use cases primarily outside of a browser (but also in
one!) which is a perfect fit for WASI.

Rust's support for WebAssembly Interface Types comes through the
[`wasm-bindgen` project](https://github.com/rustwasm/wasm-bindgen). When
using `wasm-bindgen` as a crate, though, it requires also executing the
matching CLI `wasm-bindgen` tool on the final WebAssembly binary. The
`cargo wasi` subcommand will automatically find and install the matching binary
to run on your WASI WebAssembly file. Using `cargo wasi` will also
automatically configure `wasm-bindgen` to enable interface types support.

## Deleting DWARF debuginfo in release mode

The standard Rust toolchain, following the convention of all platforms, ships
an optimized standard library for the `wasm32-wasi` target that contains DWARF
debug information. This is typically what you want in debug builds to have
a better debugging experience for the standard library, but release builds of
WebAssembly are often focused on size and disable debug information by default.
Following standard practice for all targets the Rust toolchain will by default
include the standard library's DWARF debug information in the final `*.wasm`
file, but `cargo wasi` will strip it out.

Note that this strip only happens if your build disables debuginfo in a release
executable. If you enable debuginfo in the release executable, then `cargo wasi`
will not strip out the dwarf debug information.

## Demangling Rust symbols in the `name` section

WebAssembly's [`name` custom
section](http://webassembly.github.io/spec/core/appendix/custom.html#name-section)
is present in debug and release builds of WebAssembly binaries, but Rust
symbols, like all other platforms, are mangled! This means that instead of
`main` you'll see `_ZN4main20h...`, very long symbol names.

The `cargo wasi` toolchain will ensure that all Rust symbol names in the `name`
section are demangled into a more human-readable form, improving the debugging
experience when using native tooling.

## Configuration for the `name` and `producers` Custom Sections

WebAssembly has a [`name` custom
section](http://webassembly.github.io/spec/core/appendix/custom.html#name-section)
for providing debug names to functions/locals/etc which assist in debugging
WebAssembly modules. Additionally a [`producers` custom
section](https://github.com/WebAssembly/tool-conventions/blob/master/ProducersSection.md)
is typically used to collect metadata about tools used to produce a WebAssembly
binary.

These two sections are emitted by default into all `*.wasm` binaries (including
release builds). Using `cargo wasi`, though, you can ensure they're
deleted from release builds in your `Cargo.toml`:

```toml
[package.metadata]
wasm-name-section = false
wasm-producers-section = false
```

More information about configuration can be found [in the reference](config.md)
