Texture2D scene : register(t0);
SamplerState sampleScene : register(s0);
cbuffer Grade : register(b0) {
    float4 grade;
    float4 tint;
    float4 unused0;
    float4 unused1;
};
float4 main(float4 position : SV_Position, float2 uv : TEXCOORD0) : SV_Target {
    float4 color = scene.Sample(sampleScene, uv);
    float luminance = dot(color.rgb, float3(0.2126, 0.7152, 0.0722));
    float3 rgb = lerp(luminance.xxx, color.rgb, grade.y);
    rgb = (rgb - 0.5) * grade.x + 0.5;
    return float4(saturate(rgb * grade.z * tint.rgb), color.a);
}
