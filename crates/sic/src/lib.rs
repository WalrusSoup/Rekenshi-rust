//! RE_Kenshi SIC container compatibility, implemented with bounded safe Rust.
//! Header: ShIt, LE width/height, pixel/filter/channel-layout bytes, padding.
#![forbid(unsafe_code)]
pub const LIMIT: usize = 256 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Format {
    Bgr = 0,
    Bgra = 1,
    R8 = 2,
    R16 = 3,
}
impl Format {
    pub fn bytes(self) -> usize {
        match self {
            Self::Bgr => 3,
            Self::Bgra => 4,
            Self::R8 => 1,
            Self::R16 => 2,
        }
    }
    pub fn ogre(self) -> u32 {
        match self {
            Self::Bgr => 10,
            Self::Bgra => 12,
            Self::R8 => 1,
            Self::R16 => 2,
        }
    }
}
#[derive(Debug)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub format: Format,
    pub pixels: Vec<u8>,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceStamp {
    source_size: u64,
    modified_secs: u64,
    modified_nanos: u32,
}
impl SourceStamp {
    pub fn capture(path: &std::path::Path) -> Result<Self, String> {
        let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
        let time = metadata
            .modified()
            .map_err(|e| e.to_string())?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            source_size: metadata.len(),
            modified_secs: time.as_secs(),
            modified_nanos: time.subsec_nanos(),
        })
    }
}
pub fn load_image(path: &std::path::Path) -> Result<Image, String> {
    use image::DynamicImage;
    let mut reader = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(32768);
    limits.max_image_height = Some(32768);
    limits.max_alloc = Some(LIMIT as u64);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|e| e.to_string())?;
    let (width, height) = (decoded.width(), decoded.height());
    let (format, pixels) = match decoded {
        DynamicImage::ImageLuma16(value) => (
            Format::R16,
            value
                .into_raw()
                .into_iter()
                .flat_map(u16::to_le_bytes)
                .collect(),
        ),
        DynamicImage::ImageLuma8(value) => (Format::R8, value.into_raw()),
        DynamicImage::ImageRgb8(value) => {
            let mut pixels = value.into_raw();
            for pixel in pixels.chunks_exact_mut(3) {
                pixel.swap(0, 2);
            }
            (Format::Bgr, pixels)
        }
        DynamicImage::ImageRgb16(_)
        | DynamicImage::ImageRgba16(_)
        | DynamicImage::ImageLumaA16(_)
        | DynamicImage::ImageRgb32F(_)
        | DynamicImage::ImageRgba32F(_) => {
            return Err(
                "SIC supports 8-bit color and 16-bit grayscale; refusing a lossy conversion".into(),
            );
        }
        value => {
            let mut pixels = value.into_rgba8().into_raw();
            for pixel in pixels.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
            (Format::Bgra, pixels)
        }
    };
    dimensions(width, height, format)?;
    Ok(Image {
        width,
        height,
        format,
        pixels,
    })
}
fn dimensions(width: u32, height: u32, format: Format) -> Result<usize, String> {
    if width == 0 || height == 0 || width > 32768 || height > 32768 {
        return Err("invalid SIC dimensions".into());
    }
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(format.bytes()))
        .filter(|n| *n <= LIMIT)
        .ok_or("SIC image exceeds 256 MiB".into())
}
pub fn decode(bytes: &[u8]) -> Result<Image, String> {
    // Legacy writers leave the reserved padding byte uninitialized.
    if bytes.len() < 16 || bytes.len() > LIMIT || &bytes[..4] != b"ShIt" {
        return Err("invalid SIC header".into());
    }
    let width = u32::from_le_bytes(bytes[4..8].try_into().map_err(|_| "SIC width")?);
    let height = u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| "SIC height")?);
    let format = match bytes[12] {
        0 => Format::Bgr,
        1 => Format::Bgra,
        2 => Format::R8,
        3 => Format::R16,
        _ => return Err("unsupported SIC pixel type".into()),
    };
    let (filter, layout) = (bytes[13], bytes[14]);
    if filter > 6 || layout > 1 {
        return Err("unsupported SIC filter/channel layout".into());
    }
    let size = dimensions(width, height, format)?;
    let mut decoder = zstd::bulk::Decompressor::new().map_err(|e| e.to_string())?;
    decoder
        .set_parameter(zstd::zstd_safe::DParameter::WindowLogMax(27))
        .map_err(|e| e.to_string())?;
    let mut pixels = decoder
        .decompress(&bytes[16..], size)
        .map_err(|e| format!("SIC zstd: {e}"))?;
    if pixels.len() != size {
        return Err("SIC decompressed size differs from dimensions".into());
    }
    let (channels, unit) = if format == Format::R16 {
        (1, 2)
    } else {
        (format.bytes(), 1)
    };
    if layout == 1 && channels > 1 {
        let planar = pixels.clone();
        let samples = size / channels;
        for (i, byte) in pixels.iter_mut().enumerate() {
            *byte = planar[(i % channels) * samples + i / channels];
        }
    }
    let row = width as usize * channels;
    let mask = if unit == 2 { 65535 } else { 255 };
    for i in 0..size / unit {
        let x = (i % row) / channels;
        let y = i / row;
        let left = if i >= channels {
            sample(&pixels, i - channels, unit)
        } else {
            0
        };
        let above = if y > 0 {
            sample(&pixels, i - row, unit)
        } else {
            0
        };
        let diagonal = if x > 0 && y > 0 {
            sample(&pixels, i - row - channels, unit)
        } else {
            0
        };
        let prediction = match filter {
            0 => 0,
            1 | 6 => left,
            _ if y == 0 => left,
            _ if x == 0 => above,
            2 => (left + above) / 2,
            3 => (left + above + diagonal) / 3,
            4 => {
                let estimate = left as i32 + above as i32 - diagonal as i32;
                let mut best = left;
                for candidate in [above, diagonal] {
                    if (estimate - candidate as i32).abs() < (estimate - best as i32).abs() {
                        best = candidate;
                    }
                }
                best
            }
            5 => (left as i32 + above as i32 - diagonal as i32).clamp(0, mask as i32) as u32,
            _ => return Err("SIC filter".into()),
        };
        let residual = sample(&pixels, i, unit);
        let value = if filter == 6 {
            residual ^ prediction
        } else {
            residual.wrapping_add(prediction) & mask
        };
        put(&mut pixels, i, unit, value);
    }
    Ok(Image {
        width,
        height,
        format,
        pixels,
    })
}
fn sample(bytes: &[u8], index: usize, unit: usize) -> u32 {
    if unit == 1 {
        bytes[index] as u32
    } else {
        u16::from_le_bytes([bytes[index * 2], bytes[index * 2 + 1]]) as u32
    }
}
fn put(bytes: &mut [u8], index: usize, unit: usize, value: u32) {
    if unit == 1 {
        bytes[index] = value as u8
    } else {
        bytes[index * 2..index * 2 + 2].copy_from_slice(&(value as u16).to_le_bytes());
    }
}
pub fn encode(image: &Image, level: i32) -> Result<Vec<u8>, String> {
    if image.pixels.len() != dimensions(image.width, image.height, image.format)?
        || !(1..=19).contains(&level)
    {
        return Err("invalid SIC pixels/compression level".into());
    }
    let mut payload = zstd::bulk::compress(&image.pixels, level).map_err(|e| e.to_string())?;
    let mut filter = 0;
    let (channels, unit) = if image.format == Format::R16 {
        (1, 2)
    } else {
        (image.format.bytes(), 1)
    };
    let mut residual = image.pixels.clone();
    for i in channels..image.pixels.len() / unit {
        let delta =
            sample(&image.pixels, i, unit).wrapping_sub(sample(&image.pixels, i - channels, unit));
        put(&mut residual, i, unit, delta);
    }
    let candidate = zstd::bulk::compress(&residual, level).map_err(|e| e.to_string())?;
    if candidate.len() < payload.len() {
        payload = candidate;
        filter = 1;
    }
    let mut result = b"ShIt".to_vec();
    result.extend(image.width.to_le_bytes());
    result.extend(image.height.to_le_bytes());
    result.extend([image.format as u8, filter, 0, 0]);
    result.extend(payload);
    Ok(result)
}
/// Reconstruct an uncompressed top-origin TGA for the original image loader.
pub fn tga(image: &Image) -> Result<Vec<u8>, String> {
    if image.width > 65535 || image.height > 65535 || image.format == Format::R16 {
        return Err("TGA sidecars support BGR, BGRA and R8".into());
    }
    let mut result = vec![0; 18];
    result[2] = if image.format == Format::R8 { 3 } else { 2 };
    result[12..14].copy_from_slice(&(image.width as u16).to_le_bytes());
    result[14..16].copy_from_slice(&(image.height as u16).to_le_bytes());
    result[16] = (image.format.bytes() * 8) as u8;
    result[17] = 0x20 | if image.format == Format::Bgra { 8 } else { 0 };
    result.extend(&image.pixels);
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_pixel_types_round_trip() {
        for format in [Format::Bgr, Format::Bgra, Format::R8, Format::R16] {
            let pixels = (0..17 * 11 * format.bytes())
                .map(|i| (i.wrapping_mul(29)) as u8)
                .collect();
            let input = Image {
                width: 17,
                height: 11,
                format,
                pixels,
            };
            let output = decode(&encode(&input, 3).unwrap()).unwrap();
            assert_eq!(output.pixels, input.pixels);
            assert_eq!(output.format, format);
        }
    }
    #[test]
    fn malformed_and_oversized_files_fail() {
        for bytes in [
            vec![],
            vec![0; 16],
            b"ShIt\xff\xff\xff\xff\xff\xff\xff\xff\0\0\0\0".to_vec(),
        ] {
            assert!(decode(&bytes).is_err());
        }
        let input = Image {
            width: 3,
            height: 2,
            format: Format::Bgr,
            pixels: vec![1; 18],
        };
        let mut encoded = encode(&input, 1).unwrap();
        encoded.pop();
        assert!(decode(&encoded).is_err());
    }
}
