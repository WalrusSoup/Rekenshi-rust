# __ID__

Edit `src/implementation.rs`; the SDK supplies DLL exports, menu controls,
settings persistence and managed callback cleanup. Keep native access behind
verified adapters. `src/lib.rs` only invokes `export_plugin!`.

Build from this directory with `cargo build --release`.
Distribute `target/release/__DLL__` for Windows x64.
Players copy the DLL into `Kenshi/RE_Kenshi_Rust/plugins/` and restart Kenshi.
Controls appear in Options -> REKENSHI-RUST. Settings live in
`RE_Kenshi_Rust/settings/__ID__.toml`. No host registry changes are needed.

The SDK is a local dependency: keep its checkout available while building.
See the host repository's `docs/plugin-sdk.md`, `docs/plugin-callbacks.md`
and `docs/graphics.md` for contracts and supported capabilities.
