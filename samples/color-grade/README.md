# Color-grade sample

The former built-in grading effect is preserved here as an optional sample.
Build with `cargo build --release -p rekenshi-color-grade-dll`, copy
`target/release/rekenshi_color_grade_sample.dll` into `Kenshi/RE_Kenshi_Rust/plugins/`
and restart. See [sample installation](../README.md). The DLL wrapper under
`dll/` exports the SDK entry point; the parent library shares policy with the
legacy compatibility adapter. Its enable, contrast, saturation and brightness
controls apply live in Options and persist in `settings/color-grade.toml`.
Installing the DLL enables the sample by default.

`src/plugin.rs` owns a safe `PostProcess` and declares a graphics dependency.
`src/color_grade.hlsl` adjusts luminance saturation, contrast, brightness and RGB
tint after tonemapping. Optional `color-grade.toml` supplies tint `[1.02,1,0.98]`
and priority 100; missing files take defaults. Valid tint channels are finite
and in 0..4. The safe implementation validates before changing its active pass.
Disabling or stopping removes the pass. The native module also preserves the
external C graphics-service demonstration used by `compat/color-grade`.

This is a color-only pass with no depth/lighting changes. The newer
[Kenshi 2 style sample](../kenshi2-style/README.md) is a fuller safe Rust example.
`scripts/test-enhancements.py` checks legacy service ownership and removal;
SDK/runtime tests cover settings and owned pass lifecycle. No screenshot
or FPS benefit is implied by these integration tests.
