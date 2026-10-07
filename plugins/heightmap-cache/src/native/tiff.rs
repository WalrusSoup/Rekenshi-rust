//! Checked layout for the game's uncompressed, little-endian 16-bit TIFF.
#![forbid(unsafe_code)]
use std::collections::BTreeMap;
use std::ops::Range;

pub(super) struct Layout {
    pub width: usize,
    pub height: usize,
    rows: Vec<Range<usize>>,
}
fn short(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let data = bytes
        .get(offset..offset.checked_add(2).ok_or("TIFF overflow")?)
        .ok_or("truncated TIFF")?;
    Ok(u16::from_le_bytes([data[0], data[1]]))
}
fn long(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let data = bytes
        .get(offset..offset.checked_add(4).ok_or("TIFF overflow")?)
        .ok_or("truncated TIFF")?;
    Ok(u32::from_le_bytes(
        data.try_into().map_err(|_| "truncated TIFF")?,
    ))
}
impl Layout {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.get(..4) != Some(b"II\x2a\0") {
            return Err("heightmap requires a classic little-endian TIFF".into());
        }
        let ifd = long(bytes, 4)? as usize;
        let count = short(bytes, ifd)? as usize;
        if count > 256 {
            return Err("too many TIFF fields".into());
        }
        let mut fields = BTreeMap::new();
        for i in 0..count {
            let entry = ifd + 2 + i * 12;
            let tag = short(bytes, entry)?;
            if ![256, 257, 258, 259, 273, 277, 278, 279, 284, 339].contains(&tag) {
                continue;
            }
            let kind = short(bytes, entry + 2)?;
            let len = long(bytes, entry + 4)? as usize;
            if len == 0 || len > 65536 {
                return Err("invalid TIFF field length".into());
            }
            let unit = match kind {
                3 => 2,
                4 => 4,
                _ => return Err("unsupported TIFF field type".into()),
            };
            let start = if len * unit <= 4 {
                entry + 8
            } else {
                long(bytes, entry + 8)? as usize
            };
            let mut values = Vec::with_capacity(len);
            for n in 0..len {
                values.push(if unit == 2 {
                    short(bytes, start + n * unit)? as u32
                } else {
                    long(bytes, start + n * unit)?
                });
            }
            if fields.insert(tag, values).is_some() {
                return Err("duplicate TIFF field".into());
            }
        }
        if long(bytes, ifd + 2 + count * 12)? != 0 {
            return Err("multiple TIFF images unsupported".into());
        }
        let scalar = |tag: u16, default: Option<u32>| -> Result<u32, String> {
            match fields.get(&tag).map(Vec::as_slice) {
                Some([value]) => Ok(*value),
                None => default.ok_or_else(|| format!("missing TIFF field {tag}")),
                _ => Err(format!("invalid TIFF field {tag}")),
            }
        };
        let width = scalar(256, None)? as usize;
        let height = scalar(257, None)? as usize;
        if width == 0 || height == 0 || width > 32768 || height > 32768 {
            return Err("heightmap dimensions outside supported bounds".into());
        }
        if scalar(258, None)? != 16
            || scalar(259, Some(1))? != 1
            || scalar(277, Some(1))? != 1
            || scalar(284, Some(1))? != 1
            || scalar(339, Some(1))? != 1
        {
            return Err("heightmap must be uncompressed unsigned 16-bit grayscale".into());
        }
        let rows_per_strip = scalar(278, None)? as usize;
        if rows_per_strip == 0 {
            return Err("zero TIFF strip height".into());
        }
        let offsets = fields.get(&273).ok_or("missing TIFF strip offsets")?;
        let counts = fields.get(&279).ok_or("missing TIFF strip lengths")?;
        if offsets.len() != height.div_ceil(rows_per_strip) || counts.len() != offsets.len() {
            return Err("TIFF strip count mismatch".into());
        }
        let stride = width * 2;
        let mut rows = Vec::with_capacity(height);
        for (i, (&start, &count)) in offsets.iter().zip(counts).enumerate() {
            let start = start as usize;
            let count = count as usize;
            let row_count = rows_per_strip.min(height - i * rows_per_strip);
            let required = row_count * stride;
            if count < required || start.checked_add(count).is_none_or(|end| end > bytes.len()) {
                return Err("TIFF strip extends outside its file".into());
            }
            for row in 0..row_count {
                let begin = start + row * stride;
                rows.push(begin..begin + stride);
            }
        }
        Ok(Self {
            width,
            height,
            rows,
        })
    }
    pub fn row<'a>(&self, bytes: &'a [u8], x: usize, y: usize, width: usize) -> Option<&'a [u8]> {
        if width == 0 || x.checked_add(width)? > self.width {
            return None;
        }
        let row = self.rows.get(y)?;
        bytes.get(row.start + x * 2..row.start + (x + width) * 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let fields = [
            (256, 2),
            (257, 2),
            (258, 16),
            (259, 1),
            (273, 110),
            (277, 1),
            (278, 2),
            (279, 8),
        ];
        let mut bytes = b"II\x2a\0\x08\0\0\0".to_vec();
        bytes.extend_from_slice(&(fields.len() as u16).to_le_bytes());
        for (tag, value) in fields {
            bytes.extend_from_slice(&(tag as u16).to_le_bytes());
            bytes.extend_from_slice(&4u16.to_le_bytes());
            bytes.extend_from_slice(&1u32.to_le_bytes());
            bytes.extend_from_slice(&(value as u32).to_le_bytes());
        }
        bytes.extend_from_slice(&0u32.to_le_bytes());
        for pixel in [0u16, 65535, 1234, 40000] {
            bytes.extend_from_slice(&pixel.to_le_bytes());
        }
        bytes
    }
    #[test]
    fn rows_preserve_pixels_and_reject_out_of_bounds() {
        let bytes = fixture();
        let layout = Layout::parse(&bytes).unwrap();
        assert_eq!(layout.row(&bytes, 0, 0, 2), Some(&bytes[110..114]));
        assert_eq!(layout.row(&bytes, 1, 1, 1), Some(&bytes[116..118]));
        for (x, y, w) in [(2, 0, 1), (0, 2, 1), (usize::MAX, 0, 2), (0, 0, 0)] {
            assert!(layout.row(&bytes, x, y, w).is_none());
        }
    }
    #[test]
    fn truncation_compression_and_bad_offsets_are_rejected() {
        let good = fixture();
        for length in [0, 4, 8, 90, 109, 117] {
            assert!(Layout::parse(&good[..length]).is_err());
        }
        let mut bytes = good.clone();
        bytes[54..58].copy_from_slice(&5u32.to_le_bytes()); // compression
        assert!(Layout::parse(&bytes).is_err());
        bytes = good;
        bytes[66..70].copy_from_slice(&u32::MAX.to_le_bytes()); // offset
        assert!(Layout::parse(&bytes).is_err());
    }
}
