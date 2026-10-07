# Weather effects and rain refinement

The core ships **Tone down weather effects** under Options → REKENSHI-RUST,
enabled by default. It softens vanilla rain, dust storms, ash and particle fog to
35% brightness (additive effects) or opacity (alpha-blended effects). Changes
apply live and save in `modpack.toml` as `weather.tone_down`. It does not change
weather damage, visibility rules, rain collection or other simulation behavior.
The original billboard geometry and regional colors remain.

Shared smoke textures are classified using the particle system's exact template
name, so fire and ordinary smoke bypass the weather reduction. Custom template
names bypass storm reduction; the five known rain materials remain recognized.
Particle fog is softened; global atmospheric fog and fullscreen shaders are not
changed. Emission counts are unchanged; no FPS improvement is claimed.

Optional thin streaks and camera fade live in an independent DLL under
`samples/rain-refinement`; see [sample setup](../samples/README.md). The sample
uses the SDK's safe `RainRendering` handle. The host owns the rendering adapter;
both controls coexist without competing hooks. Rebuild older rain-refinement
DLLs with this SDK before using them with the shared adapter.

The SDK exports its three controls in Options: enable, visibility (default 0.25),
and streak size (default 1.0). Changes apply on the next particle render.
Disabling removes only the sample style. The built-in weather checkbox remains independent.

Vanilla `Kenshi_Rain_RED.pu` requests 1×10 metre rain billboards; heavy red rain
uses 0.85×4 metres. Their textures and additive blending produce large bright
curtains. This plugin scales width to 18%, caps it at 0.12 metres, scales length
to 45% and caps it at 1.8 metres. The size control multiplies those dimensions.
RGB brightness is multiplied by visibility and a smooth proximity fade between
3 and 12 metres from the camera. Regional texture colors remain, including red.

The sample style recognizes only the five native rain materials. It scopes changes
to ParticleUniverse's BillboardRenderer submissions, temporarily changes set
defaults while vertices are generated, and adjusts copies of explicit-size
billboards. It restores defaults even during unwinding. Original particles,
materials, weather strengths, rain collectors, damage, audio and scripts are
untouched. Splash, dust, blood, fire and other billboard effects bypass the optional rain style.
Local-space particles are transformed by their parent node before camera fade.
There are no persistent native object pointers or per-drop allocations.

Supported binaries:

- OgreMain_x64.dll SHA256 `7D3888D21398647519F10A89B1F300D8606F761AD159E6021A797B1B9B3DF5F6`
- Plugin_ParticleUniverse_x64.dll SHA256 `24705F105A13C596179964082B89CC5997FE70091CECC030A4CC380E52AD61A0`

Verified exports: BillboardRenderer::_updateRenderQueue RVA 0xca960,
ParticleRenderer::getParentTechnique 0x6b5d0, ParticleTechnique::getMaterialName
0x931c0; Ogre BillboardSet::injectBillboard 0x7a940 and setDefaultDimensions
0x7a080. The native renderer stores its set at 0xd8. Billboard is 0x58 bytes,
with own-size flag at 0, width/height 0x14/0x18, position 0x1c, RGB(A) 0x40.
Opaque copies preserve padding and avoid reading unused uninitialized dimensions.

`scripts/test-weather.py` runs the real detours and exported getters/setters in
a staged process with capture callbacks replacing the two original render bodies.
It checks the built-in reduction, shared smoke exclusions, sample composition, dimensions, colors, camera fade, explicit-size drops, immediate controls,
unmodified input, restored defaults, unrelated effects and shutdown/restart.
Rust tests check fade boundaries and transformed positions. These checks validate
the adapter, not in-game appearance: visual tuning still needs a rain-save test.

This first pass retains the native billboard renderer and original rain textures.
It does not add depth-based soft particles, wet surfaces or a new precipitation
simulation. Emission counts are unchanged; no FPS improvement is claimed.
With other weather mods it refines the resulting rain, so already-reduced effects
may need a higher visibility setting. Custom material names bypass the plugin.
