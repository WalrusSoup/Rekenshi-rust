# Live squad inspector

An independent SDK DLL that exports the player's live roster to readable JSON.

```powershell
cargo build --release -p rekenshi-squad-inspector
```

Copy `target/release/rekenshi_squad_inspector.dll` into
`Kenshi/RE_Kenshi_Rust/plugins/`, then restart Kenshi with the updated host DLL.
The plugin writes `Kenshi/RE_Kenshi_Rust/squad-inspector.json`. Open it in
an editor or read it from a dashboard; retry a read if it overlaps a rewrite.

The JSON reports `ready`, `no-world`, `waiting`, or `unsupported`, a diagnostic
message, capability flags, and character names, IDs, XYZ positions, squad metadata,
selection, health and stats when available. Unknown fields are `null`; IDs are
strings. It polls at 250 ms and writes changed content at most once per second.
The inspector never moves or changes characters.

See [the gameplay SDK](../../docs/gameplay.md) for querying snapshots, checking
native support and submitting player-character commands with result tickets.
