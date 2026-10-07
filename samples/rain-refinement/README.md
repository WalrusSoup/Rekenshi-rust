# Rain-refinement sample

The former built-in rain effect is an independent SDK DLL. Build with
`cargo build --release -p rekenshi-rain-refinement`, copy
`target/release/rekenshi_rain_refinement.dll` into `Kenshi/RE_Kenshi_Rust/plugins/`
and restart. See [sample installation](../README.md). Installing the DLL enables
the effect by default. Checkbox, visibility and size controls apply live in
Options and persist in `settings/rain-refinement.toml`.

`src/plugin.rs` declares controls and owns a safe SDK `RainRendering` handle.
The host fingerprints Ogre and ParticleUniverse and owns the shared hooks.
It changes only five recognized rain materials: narrower/shorter billboards,
brightness reduction and a camera proximity fade. It submits copies, restores
native defaults and leaves unrelated particles untouched. Original regional
colors remain, including red; this does not replace rain textures or blending.

See [the weather adapter notes](../../docs/weather.md) for bounds, native
fingerprints, ownership and limitations. `scripts/test-weather.py` explicitly
loads this sample DLL and exercises real detours with captured submissions,
including source restoration, camera fade, unrelated particles and stop/restart.
In-game appearance needs a world with rain; these checks do not judge the art.

The built-in **Tone down weather effects** checkbox remains independent. Its
35% visibility reduction multiplies this sample's visibility without shrinking
streaks again. Disable either control live. Rebuild an older sample DLL with the
current SDK; older private detours conflict with the shared host adapter.
