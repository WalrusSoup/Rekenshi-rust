// Independently authored shader: no ReShade preset/shader assets are bundled.
Texture2D scene : register(t0);
SamplerState sampleScene : register(s0);
cbuffer Style : register(b0) {
    float4 ink;       // strength, width in pixels, threshold, softness
    float4 grade;     // contrast, saturation, exposure stops, palette blend
    float4 finish;    // palette levels, sharpen, warmth, reserved
    float4 reserved;
};
float luminance(float3 color) { return dot(color, float3(0.2126, 0.7152, 0.0722)); }
float4 main(float4 position : SV_Position, float2 uv : TEXCOORD0) : SV_Target {
    uint width, height;
    scene.GetDimensions(width, height);
    float2 pixel = ink.y / float2(width, height);
    float4 center = scene.Sample(sampleScene, uv);
    float3 tl = scene.Sample(sampleScene, uv + pixel * float2(-1,-1)).rgb;
    float3 t  = scene.Sample(sampleScene, uv + pixel * float2( 0,-1)).rgb;
    float3 tr = scene.Sample(sampleScene, uv + pixel * float2( 1,-1)).rgb;
    float3 l  = scene.Sample(sampleScene, uv + pixel * float2(-1, 0)).rgb;
    float3 r  = scene.Sample(sampleScene, uv + pixel * float2( 1, 0)).rgb;
    float3 bl = scene.Sample(sampleScene, uv + pixel * float2(-1, 1)).rgb;
    float3 b  = scene.Sample(sampleScene, uv + pixel * float2( 0, 1)).rgb;
    float3 br = scene.Sample(sampleScene, uv + pixel * float2( 1, 1)).rgb;
    float gx = luminance(tr + 2*r + br - tl - 2*l - bl) * 0.25;
    float gy = luminance(bl + 2*b + br - tl - 2*t - tr) * 0.25;
    float edge = smoothstep(ink.z, ink.z + ink.w, length(float2(gx, gy)));
    float3 color = center.rgb + finish.y * (center.rgb - (t+l+r+b) * 0.25);
    color *= exp2(grade.z);
    color = lerp(luminance(color).xxx, color, grade.y);
    color = (color - 0.5) * grade.x + 0.5;
    color *= float3(1 + 0.12*finish.z, 1 + 0.03*finish.z, 1 - 0.12*finish.z);
    color = saturate(color);
    float levels = max(2, floor(finish.x + 0.5)) - 1;
    color = lerp(color, floor(color * levels + 0.5) / levels, grade.w);
    color *= 1 - ink.x * edge;
    return float4(saturate(color), center.a);
}
