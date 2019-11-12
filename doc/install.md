# Installation

To install `cargo-wasi` you'll first want to [install Rust
itself](https://www.rust-lang.org/tools/install), which you'll need anyway for
building Rust code! Once you've got Rust installed you can install `cargo-wasi`
with:

```
$ cargo install cargo-wasi
```

This will install a precompiled binary for most major platforms or install from
source if we don't have a precompiled binary for your platform. If you would
like to see a precompiled binary for your platform [please file an
issue](https://github.com/bytecodealliance/cargo-wasi/issues/new)!.

To verify that your installation works, you can execute:

```
$ cargo wasi --version
```

and that should print both the version number as well as git information about
where the binary was built from.

Now that everything is set, let's build some code for wasi!
