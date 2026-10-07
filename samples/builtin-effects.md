# Our plugin inventory

These entries describe this repository's implementations, not promises of parity
with every optimization in the original RE_Kenshi. The registry and settings are
the source of truth; [performance notes](../docs/performance.md) record remaining
work and supported executable fingerprints.

| Plugin | Location | Default behavior | Adapter / scope |
| --- | --- | --- | --- |
| shader-cache | `plugins/shader-cache` | Enabled | Bounded disk cache for successful D3DCompile results, dependency-aware for the verified Ogre include handler; unknown handlers bypass. |
| heightmap-cache | `plugins/heightmap-cache` | Disabled | Cached supported terrain-map reads; foreign maps bypass. Native TIFF reader adapter. |
| collider-cache | `plugins/collider-cache` | Disabled | Supported PhysX collection caching; executable fingerprint checked. |
| dds-mip-skip | `plugins/dds-mip-skip` | Disabled | Supported native DDS listener optimization; executable fingerprint checked. |
| compressed-textures | `plugins/compressed-textures` | Disabled | SIC texture decode and offline `texture-pack` conversion; does not silently repack game assets. |
| weather | `src/weather` | Enabled | Tone down weather effects checkbox; additive brightness / alpha opacity at 35% for known rain, dust, ash and particle fog. Shared renderer supports optional rain DLLs. |
| dark-ui | `src/ui/dark_mode` | Enabled | Darkens original Kenshi frames and brightens text; live frame brightness control. |
| graphics | `plugins/graphics` | Infrastructure mounted | World LDR post-tonemap pass service. With no enabled effects, no effect pass runs. |
| kenshi2-style | `samples/kenshi2-style` | DLL optional; checkbox off | Original color-based ink, desert grading, palette simplification and sharpening. Safe `PostProcess` sample. |
| color-grade | `samples/color-grade` | DLL absent from core install | Our former built-in effect. Contrast/saturation/brightness, optional RGB tint and priority in `color-grade.toml`. Safe policy plus legacy C service adapter. |
| rain-refinement | `samples/rain-refinement` | DLL absent from core install | Our former built-in weather effect. Smaller rain billboards and scoped brightness/proximity fade for five verified rain materials. [Native details](../docs/weather.md). |

The last three are optional visual samples. Weather reduction, dark UI and
performance optimizations are first-class pack plugins. Unsupported native builds
show a reason in the menu instead of applying guessed offsets.

## Composition examples

Performance plugins use `.register(ShaderCache)` / `.register(HeightmapCache)`.
They own their settings, native adapters and stop behavior. New behavioral plugins
can use managed `context.host` callbacks without writing a detour. See
[callbacks](../docs/plugin-callbacks.md).

Independent graphics DLLs declare `requires() == &["graphics"]` and use
`rekenshi_plugin_sdk::export_plugin!(YourEffect)`. Folder discovery loads them
after the core graphics backend. The [SDK guide](../docs/plugin-sdk.md) covers
creation, building and distribution without editing the host registry.

An effect owns `Option<PostProcess>` and declares settings once. `start` delegates
to `update`; an enabled update creates or updates a pass, a disabled update calls
`stop`, and `stop` removes it. See the [complete sample](kenshi2-style/src/effect.rs).
Multiple effects compose in priority order; each pass sees the previous pass's
color output. This does not grant simulation hooks or an Ogre material editor.

The rain sample uses a safe `RainRendering` handle to configure thin rain streaks
and camera fade. The host owns and fingerprints the Ogre/ParticleUniverse hooks,
scopes rendering changes, restores defaults and submits billboard copies.
The shared seam lets the built-in weather checkbox and independent rain plugins
coexist. Regional colors, weather simulation and native blending remain.
