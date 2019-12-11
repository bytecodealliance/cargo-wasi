# Contributing to `cargo-wasi`

This section contains instructions on how to get this project up and running for
development. Source code for this project lives on GitHub at
https://github.com/bytecodealliance/cargo-wasi.

## Prerequisites

1. The `cargo-wasi` subcommand is written in Rust, so you'll want [Rust
   installed](https://www.rust-lang.org/tools/install)

2. Running tests requires [`wasmtime` is installed and in
   `$PATH`](https://wasmtime.dev) or an existing runtime provided via `CARGO_TARGET_WASM32_WASI_RUNNER`.

## Getting the code

You'll clone the code via `git`:

```
$ git clone https://github.com/bytecodealliance/cargo-wasi
```

## Testing changes

We'd like tests ideally to be written for all changes. Test can be run via:

```
$ cargo test
```

You'll be adding tests primarily to `tests/tests/*.rs`.

## Submitting changes

Changes to `cargo-wasi` are managed through Pull Requests, and anyone is
more than welcome to submit a pull request! We'll try to get to reviewing it or
responding to it in at most a few days.

## Code Formatting

Code is required to be formatted with the current Rust stable's `cargo fmt`
command. This is checked on CI.

## Continuous Integration

The CI for the `cargo-wasi` repository is relatively significant. It tests
changes on Windows, macOS, and Linux. It also performs a "dry run" of the
release process to ensure that release binaries can be built and are ready to be
published.

## Publishing a New Version

Publication of this crate is entirely automated via CI. A publish happens
whenever a tag is pushed to the repository, so to publish a new version you'll
want to make a PR that bumps the version numbers (see the `bump.rs` scripts in
the root of the repository), merge the PR, then tag the PR and push the tag.
That should trigger all that's necessary to publish all the crates and binaries
to crates.io.
