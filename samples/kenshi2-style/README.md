# Kenshi 2 style graphics sample

Reference: Wugonorama's [Kenshi 2 Style ReShade](https://www.nexusmods.com/kenshi/mods/534),
inspired by Kenshi 2 concept art and based on Desertpunk. Its comic outlines and
fog-aware depth effects are the visual reference; its files and assets are not
bundled here. That preset uses ReShade and an SSAO companion, rather than requiring
RE_Kenshi. This sample is independently authored, not an exact preset conversion.

## Install

Build with `cargo build --release -p rekenshi-kenshi2-style`, copy
`target/release/rekenshi_kenshi2_style.dll` into `Kenshi/RE_Kenshi_Rust/plugins/`
and restart. The installer can do this with `-IncludeKenshi2Sample`.
No host registry edits or sample selector are needed.

## Implementation

`src/lib.rs` contains the SDK export macro; `src/effect.rs` implements the safe Rust `Plugin` interface and owns a `PostProcess`.
It declares a graphics dependency, menu controls, live updates and cleanup. No
raw pointers, detour entry points, export names or GPU objects appear in the
sample. The backend owns shader compilation, scene copies, resizing and state
restoration. A disabled sample creates no shader or render pass.

`src/style.hlsl` combines nine scene-color samples in one post-tonemap pass:

1. Compute Sobel luminance edges and darken them with adjustable ink strength.
2. Sharpen gently, adjust exposure/contrast/saturation and warm the desert palette.
3. Blend a simplified palette while preserving the scene alpha.

Default settings are ink 0.55, width 1 pixel, contrast 1.06, saturation 0.9,
exposure 0 stops, palette blend 0.15, 16 palette levels, sharpening 0.18 and
warmth 0.08. Priority 200 places this after the optional old color-grade sample.
These are our own starting values; the preset's exact parameters were unavailable.

Enable **Kenshi 2 style sample** under **Options → REKENSHI-RUST** after loading a
world. Width is measured in output pixels, so it stays consistent on resize.
Disabling removes the pass. All parameters apply live. Settings are validated
and persisted by the SDK in `RE_Kenshi_Rust/settings/kenshi2-style.toml`, and shader compilation failure leaves other plugins
available. To tune a gentler look, reduce ink and palette simplification first.

## Limits and verification

The public SDK supplies LDR scene color, not depth, normals or motion history.
Edges therefore follow visible color contrast, including texture details. They
cannot detect same-color silhouettes or reproduce depth outlines, MXAO, ambient
occlusion, depth of field, fog-distance rejection or the reference's depth haze.
Strong settings can darken clouds, fog boundaries and texture detail; compare
bright desert, interiors, sandstorms and night scenes by hand.

One scene copy and fullscreen draw run per registered pass. This shader uses nine
texture samples per output pixel. GPU cost depends on resolution and hardware;
no FPS claim is made.

`scripts/test-modpack.py` exercises live enable/update/remove and settings
persistence through the real host DLL. The ignored graphics GPU test
`kenshi2_sample_identity_ink_palette_exposure_and_alpha` compiles this exact HLSL,
checks identity, edge darkening, flat-field preservation, palette rounding,
exposure clipping and alpha, and verifies restored game bindings:

```powershell
$env:REKENSHI_TEST_GAME = 'D:\SteamLibrary\steamapps\common\Kenshi'
cargo test -p rekenshi-graphics --lib kenshi2_sample -- --ignored
```
