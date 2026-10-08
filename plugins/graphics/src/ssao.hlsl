// Classic hemisphere SSAO, evaluated in Kenshi's existing lighting shader.
// The game supplies viewPos, world normal, proj, inverseView and radial depth.
// Fixed low-discrepancy directions avoid per-frame noise without temporal history.
// Radius and bias are metres; only environment lighting receives occlusion.
float reAo = 1.0;
if (reSsaoSettings.x > 0.0 && reSsaoSettings.w < 1.5)
{
    float3 reNormal = normalize(mul((float3x3)inverseView, normal));
    float3 reAxis = abs(reNormal.z) < 0.9 ? float3(0, 0, 1) : float3(0, 1, 0);
    float3 reTangent = normalize(cross(reAxis, reNormal));
    float3 reBitangent = cross(reNormal, reTangent);
    float reBlocked = 0.0;
    // Twenty-four samples balance initial GPU cost with stable contact shading.
    [unroll] for (int reI = 0; reI < 24; ++reI)
    {
        float reZ = (reI + 0.5) / 24.0;
        float reAngle = reI * 2.39996323; // golden angle spreads hemisphere samples
        float2 reDisk = float2(cos(reAngle), sin(reAngle)) * sqrt(1.0 - reZ * reZ);
        float3 reDirection = reTangent * reDisk.x + reBitangent * reDisk.y + reNormal * reZ;
        float reScale = 0.1 + 0.9 * reZ * reZ; // concentrate samples near the surface
        float3 reSample = viewPos.xyz + reDirection * (reSsaoSettings.y * reScale);
        float4 reClip = mul(proj, float4(reSample, 1));
        if (reClip.w > 0.0001)
        {
            float2 reUv = reClip.xy / reClip.w * float2(0.5, -0.5) + 0.5;
            // Match Kenshi's texCoord -= offset.zw when sampling G-buffer depth.
            reUv -= offset.zw;
            if (all(reUv > 0.0) && all(reUv < 1.0))
            {
                float reDepth = tex2Dlod(gBuf2, float4(reUv, 0, 0)).r;
                float reDistance = reDepth * pFogParams.x;
                // Zero depth is sky/empty geometry; radial distance is not Z depth.
                float reRange = saturate(1.0 - abs(reDistance - distance) / reSsaoSettings.y);
                reRange = reRange * reRange * (3.0 - 2.0 * reRange);
                reBlocked += (reDepth > 0.00001 && reDistance < length(reSample) - reSsaoSettings.z)
                    ? reRange : 0.0;
            }
        }
    }
    reAo = saturate(1.0 - reSsaoSettings.x * reBlocked / 24.0);
}
envLight.diffuse *= reAo;
envLight.specular *= reAo;
