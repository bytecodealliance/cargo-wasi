# Markdown Parsing in WebAssembly

This example uses wasm interface type (through `#[wasm_bindgen]`) to generate a
WebAssembly module that exposes a `render` function which renders a markdown
string.

Note that this example currently requires the `--release` flag to be built.
