# Independent shader-source policy

Build with `cargo build --release -p rekenshi-source-transform`. Install
`target/release/rekenshi_source_transform.dll` through the normal plugin workflow.
The host must have its public shader-source provider active (bundled shader cache,
or the compatibility shader-cache DLL). The consumer can register while that
provider is absent; it remains pending until a compatible provider publishes.

This sample depends only on the public `rekenshi-plugin-sdk` and `rekenshi-plugin-api` crates. During `register`, it owns a source
transform for `rekenshi.shader.source@1`, identified by
`example.source-transform@1`. It accepts only `main`/`ps_5_0` sources with the
`// example-source-transform` marker and exactly one `float4(1,0,0,1)` anchor,
which it changes to green. Other sources pass through. A deliberate
`// reject-source-transform` marker exercises original-source fallback.

Keep the Registration handle in plugin state. Settings remove/re-register it;
shutdown quiesces calls before releasing local callback captures. `late()` reports
earlier broker-observed compilations: existing shader objects need recompilation
or process restart. A false flag cannot rule out compilation before the native
hook was enabled. Readiness alone never promises before-first-use activation.
Native callbacks consume copied bytes and separate captured state; they do not
receive `&mut Plugin` or native GPU objects.

No FCS/data changes are required: this sample controls runtime source compilation
through the default source rule. The optional runtime examples below also exercise
shader identity, draw parameters and native loading hooks. Mesh/definition authoring
remains in the asset tools. See the
[implementation inventory](../../docs/plugin-platform-implementation.md) for the
implemented contracts and native-evidence limits.

`python scripts/test-platform.py <KenshiDir>` stages the real compiler, independent
compiler-provider DLL, this policy DLL and host under target. It verifies bytecode
changes, transformed cache identity, failure fallback, settings-driven cleanup,
late-registration reporting, stop and recreation without changing the game install.


`runtime_policy.enabled=true` in this plugin's settings file opts into copied
native examples: TIFF path/read transformation, explicit DDS mip selection,
scaled collider cloning/caching, UI colour and widget observations, particle
appearance, an `rki` image codec, shader identity/scoped b12 constants, and an
owned post-tonemap effect. These deliberately visible example values are off by
default. Set them before startup; changing the codec/example bundle requires a
restart. Native fixtures use this independently compiled DLL against the same
public providers as built-ins. Widgets deliver copied events, never native pointers.
