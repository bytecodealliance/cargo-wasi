# Running `wasm-bindgen`

> **Note**: Usage of `wasm-bindgen` and WebAssembly Interface Types is highly
> experimental, it's recommended that you expect breakage and/or surprises if
> you're using this.

The [`wasm-bindgen` project](https://github.com/rustwasm/wasm-bindgen) is
primarily targeted at JavaScript and the web, but is also becomimg the primary
experiment grounds of WebAssembly Interface Types for Rust. If you're not using
interface types you probably don't need `wasm-bindgen`, but if you're using
interface types read on!

The `cargo wasi` subcommand will automatically detect when
`wasm-bindgen`-the-crate is used in your dependency graph. When this is seen
then `cargo wasi` will download the corresponding precompiled `wasm-bindgen` CLI
binary (or `cargo install` it) and execute that over the final WebAssembly file.

Currently no configuration for `wasm-bindgen` is supported because the support
for WebAssembly Interface Types is unconditionally enabled which takes no
configuration. This aspect of `cargo wasi` is highly likely to change and get
improved over time though!
