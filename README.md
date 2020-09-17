# `adblock-rust` Dashboard

Test [`adblock-rust`](https://github.com/brave/adblock-rust) directly in the browser. Powered by WebAssembly.

`adblock-rust` Dashboard is designed to be a useful tool for adblock filter list maintainers.

## Build instructions

### Dependencies

First, follow [these instructions](https://rustup.rs) to install the `cargo` commandline tool for Rust.

Then, install `wasm-pack`:

```
cargo install wasm-pack
```

Make sure `$CARGO_HOME/bin` is added to your `$PATH`.

### Build

```
wasm-pack build --target web --out-dir ./static
```

Then, the dashboard can be served from the `static` folder, using a server implementation of your choice. For example, `python3 -m http.server`.
