//! Ambient-only SSAO policy registered through the public copied-byte source contract.
#![forbid(unsafe_code)]

const CONSTANTS: &str =
    "\ncbuffer REKenshiSsaoConstants : register(b13) { float4 reSsaoSettings; };\n";
const OCCLUSION: &str = include_str!("ssao.hlsl");

/// Transform a recognized Kenshi deferred lighting shader before cache lookup.
///
/// The graphics plugin registers this policy through the public operation broker.
/// Unknown shaders, occupied constant slots and already-patched sources bypass.
pub fn lighting_shader(source: &[u8], entry: &[u8], target: &[u8]) -> Option<Vec<u8>> {
    if entry != b"main_fs" || !target.starts_with(b"ps_") {
        return None;
    }
    // Shipped include comments can use Windows-1252; preserve their bytes.
    let proxy: Vec<u8> = source
        .iter()
        .map(|b| if b.is_ascii() { *b } else { b'?' })
        .collect();
    let text = std::str::from_utf8(&proxy).ok()?;
    if ![
        "CalcEnvironmentLight(",
        "float distance = depth * pFogParams.x;",
        "normalize(ray) * distance",
        "uniform float4x4 inverseView,",
        "uniform float4x4 proj,",
    ]
    .iter()
    .all(|anchor| text.contains(anchor))
        || text.contains("reSsaoSettings")
        || text
            .split_whitespace()
            .collect::<String>()
            .contains("register(b13)")
    {
        return None;
    }
    let lighting = "LightingData ld = (LightingData)0.0f;";
    let output = "return float4(color, 1.0f);";
    let start = text.find("float4 main_fs (")?;
    let body = start + text[start..].find('{')?;
    let mut depth = 0;
    let end = text[body..].char_indices().find_map(|(index, ch)| {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        (depth == 0).then_some(body + index + 1)
    })?;
    let function = std::str::from_utf8(&source[start..end]).ok()?;
    if function.matches(lighting).count() != 1 || function.matches(output).count() != 1 {
        return None;
    }
    let function = function.replacen(lighting, &format!("{OCCLUSION}\n\t{lighting}"), 1);
    let function = function.replacen(output,
        "if (reSsaoSettings.w > 2.5) color = normal * 0.5 + 0.5;\n\telse if (reSsaoSettings.w > 1.5) color = saturate(distance / 100.0).xxx;\n\telse if (reSsaoSettings.w > 0.5) color = reAo.xxx;\n\treturn float4(color, 1.0f);", 1);
    Some(
        [
            CONSTANTS.as_bytes(),
            &source[..start],
            function.as_bytes(),
            &source[end..],
        ]
        .concat(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_recognized_lighting_is_patched_once_and_sunlight_is_preserved() {
        let source = b"float4 main_fs () { CalcEnvironmentLight(\nfloat distance = depth * pFogParams.x;\nnormalize(ray) * distance\nuniform float4x4 inverseView,\nuniform float4x4 proj,\nLightingData ld = (LightingData)0.0f;\nreturn float4(color, 1.0f); }\nfloat4 other() { return float4(color, 1.0f); }";
        let transformed = lighting_shader(source, b"main_fs", b"ps_4_0").unwrap();
        let text = std::str::from_utf8(&transformed).unwrap();
        assert!(text.contains("envLight.diffuse *= reAo"));
        assert!(!text.contains("sunLight.diffuse *="));
        assert!(lighting_shader(&transformed, b"main_fs", b"ps_4_0").is_none());
        assert!(lighting_shader(source, b"main_vs", b"vs_4_0").is_none());
        let occupied = [source.as_slice(), b"register ( b13 )"].concat();
        assert!(lighting_shader(&occupied, b"main_fs", b"ps_4_0").is_none());
        let changed = String::from_utf8(source.to_vec())
            .unwrap()
            .replace("pFogParams.x", "anotherDepthConvention");
        assert!(lighting_shader(changed.as_bytes(), b"main_fs", b"ps_4_0").is_none());
        let legacy_comments = [b"// \x96\n".as_slice(), source, b"\n// \x97"].concat();
        let patched = lighting_shader(&legacy_comments, b"main_fs", b"ps_4_0").unwrap();
        assert!(patched.windows(4).any(|v| v == b"// \x96"));
        assert!(patched.ends_with(b"// \x97"));
    }
}
