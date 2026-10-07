Texture2D scene:register(t0);
SamplerState sampleScene:register(s0);
cbuffer Parameters:register(b0) { float4 tint;float4 p1;float4 p2;float4 p3; }
float4 main(float4 position:SV_Position,float2 uv:TEXCOORD0):SV_Target {
    float4 color=scene.Sample(sampleScene,uv);
    return float4(saturate(color.rgb*tint.rgb),color.a);
}
