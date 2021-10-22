# `adblock-rust` Dashboard

Test [`adblock-rust`](https://github.com/brave/adblock-rust) directly in the browser. Powered by WebAssembly.

`adblock-rust` Dashboard is designed to be a useful tool for adblock filter list maintainers.

## Build instructions

### Dependencies

First, follow [these instructions](https://rustup.rs) to install the `cargo` commandline tool for Rust.

Then, install the `wasm32-unknown-unknown` target triple, as well as the `wasm-bindgen-cli` tool:

```
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
```

Make sure `$CARGO_HOME/bin` is added to your `$PATH`.

### Build

```
cargo build --lib --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir static target/wasm32-unknown-unknown/release/adblock_rust_dashboard.wasm
```

Then, the dashboard can be served from the `static` folder, using a server implementation of your choice. For example, `python3 -m http.server`.
