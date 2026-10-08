# Experimental ambient occlusion

The built-in **Ambient occlusion** group adds classic hemisphere SSAO to Kenshi's
deferred environment lighting. It is disabled by default. Enable **Ambient
occlusion (experimental)** in the Mods area; start with strength 0.6, radius
1.5 metres and debug view 0. Changes apply immediately once the recognized
lighting shader has been compiled during this game session.

Debug views: 1 shows occlusion (white means open), 2 shows radial camera distance
from 0 to 100 metres, and 3 shows decoded world normals. These replace the lighting
output before downstream fog and tone mapping, so the displayed debug colours
can still be affected by those passes. Disable AO to return to ordinary lighting.

## Implementation

The graphics plugin registers its owned source policy through the public
`rekenshi.shader.source@1` operation broker before bundled providers start.
The compiler hook publishes that contract and applies the composed result.
Recognized main_fs lighting sources receive independently authored HLSL before
shader-cache hashing. This still operates with shader caching disabled. Unknown
sources, occupied b13 bindings and already patched sources bypass; a failed
injected compilation retries the original source. Original asset files are not
edited. No Dust shader code or third-party effect assets are copied.
Late registrations report missed compilations and require recompilation/restart
for existing shaders. Shader tagging and scoped draw constants remain internal;
this source migration does not yet provide independent-DLL AO draw parity.

The shader uses the game's existing view position, projection, inverse view,
decoded normal and radial depth convention. Twenty-four deterministic hemisphere
samples estimate local occlusion, with a distance falloff and four-centimetre
bias. Only environment diffuse and specular terms receive AO. Direct sunlight,
local lights and emissive terms retain their original contribution.

Successful CreatePixelShader outputs containing our reflection marker receive
private D3D11 metadata. The Draw hook binds a host-owned 16-byte constant buffer
at b13 only for tagged shaders, then restores the previous binding after the
original draw. GPU objects stay in the host; context work runs synchronously on
the intercepted rendering thread. Existing effect SDK ABI layouts are unchanged.
Device changes rebuild the small constant buffer. This first version adds no
AO render target, denoiser or temporal history.

Logs distinguish shader preparation, original-shader fallback and the first
tagged lighting draw. A preparation log alone does not prove that the game
executed the patched shader.

## Validation and limits

Tests compile the shipped lighting source with ordinary, CSM and RTW variants
under Shader Models 4 and 5. An offscreen hardware GPU fixture checks a flat
surface, contact occlusion, distant unoccluded geometry, disabled identity and
preserved direct light. Another checks shader tagging and exact b13 restoration.
The host integration fixture exercises compiler interception with caching on
and off, a deliberately failed injection with successful original-source
fallback, live menu settings and restart.

These fixtures do not establish the actual game's appearance, frame cost or
compatibility with wrappers. Validate a real scene with camera rotation, zoom,
foliage, silhouettes, interiors, fog, sky and UI. Compare with ReShade MXAO
disabled separately. The fixed kernel can show visible sampling patterns and
thin geometry can halo. Full-resolution extra depth sampling is the main GPU
cost; no timing claim has been measured in Kenshi.

The safe source transform and native resource layer follow the existing
M-FFI-TRANSLATES / M-UNSAFE separation and host ownership constraints. Per-frame
GPU work remains unprofiled in game (M-HOTPATH); this is an explicit prototype
limitation, not a guideline compliance audit.
