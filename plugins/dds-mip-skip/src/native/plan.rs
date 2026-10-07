//! Bounded DDS read planning. Unsupported formats remain with the native loader.
#![forbid(unsafe_code)]
use std::ops::Range;
pub(crate) struct Plan {
    pub header: Vec<u8>,
    pub pixels: Range<usize>,
}
fn word(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}
fn write(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
pub(crate) fn levels(name: &[u8], group: &[u8], quality: i32) -> u32 {
    if !(1..=4).contains(&quality)
        || !name.ends_with(b".dds")
        || [b"GUI".as_slice(), b"Overlaymaps", b"PLSM2", b""].contains(&group)
    {
        return 0;
    }
    let mut count = quality as u32;
    if name.windows(4).any(|s| s == b"_LO.") {
        count = count.saturating_sub(1);
    }
    if group != b"Landscape" && !name.windows(4).any(|s| s == b"_HI.") {
        count = count.saturating_sub(1);
    }
    count
}
pub(crate) fn make(bytes: &[u8], file_size: usize, dropped: u32) -> Option<Plan> {
    if bytes.get(..4)? != b"DDS "
        || word(bytes, 4)? != 124
        || word(bytes, 76)? != 32
        || dropped == 0
    {
        return None;
    }
    let flags = word(bytes, 8)?;
    let (mut width, mut height) = (word(bytes, 16)?, word(bytes, 12)?);
    if flags & 0x1007 != 0x1007
        || width == 0
        || height == 0
        || width > 32768
        || height > 32768
        || word(bytes, 112)? & 0x20fe00 != 0
        || word(bytes, 24)? > 1
    {
        return None;
    }
    let mip_count = word(bytes, 28)?;
    if word(bytes, 108)? & 0x400000 == 0 || !(2..=16).contains(&mip_count) {
        return None;
    }
    let format_flags = word(bytes, 80)?;
    let block = match bytes.get(84..88)? {
        b"DXT1" => Some(8usize),
        b"DXT2" | b"DXT3" | b"DXT4" | b"DXT5" => Some(16),
        _ if format_flags & 4 != 0 => return None,
        _ => None,
    };
    // The game's loading listener only downsizes classic DXT. Preserve native
    // texture quality for every other encoding by forwarding it unchanged.
    let block = block?;
    if !width.is_power_of_two() || !height.is_power_of_two() {
        return None;
    }
    let mut skip = 0usize;
    let mut kept = 0usize;
    let mut removed = 0u32;
    for level in 0..mip_count {
        let size = {
            (width as usize)
                .div_ceil(4)
                .checked_mul((height as usize).div_ceil(4))?
                .checked_mul(block)?
        };
        // Native DDS downscaling keeps a largest level of at least one DXT block.
        if level < dropped && level < mip_count - 1 && width > 4 && height > 4 {
            skip = skip.checked_add(size)?;
            removed += 1;
        } else {
            kept = kept.checked_add(size)?;
        }
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    if removed == 0
        || 128usize.checked_add(skip)?.checked_add(kept)? > file_size
        || kept > 256 * 1024 * 1024
    {
        return None;
    }
    let mut header = bytes.get(..128)?.to_vec();
    let width = (word(bytes, 16)? >> removed).max(1);
    let height = (word(bytes, 12)? >> removed).max(1);
    write(&mut header, 16, width);
    write(&mut header, 12, height);
    write(&mut header, 28, mip_count - removed);
    // Keep every other header field byte-identical to the native listener.
    Some(Plan {
        header,
        pixels: 128 + skip..128 + skip + kept,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn header() -> Vec<u8> {
        let mut h = vec![0; 128];
        h[..4].copy_from_slice(b"DDS ");
        for (o, v) in [
            (4, 124),
            (8, 0x21007),
            (12, 64),
            (16, 64),
            (28, 7),
            (76, 32),
            (80, 4),
            (108, 0x401008),
        ] {
            write(&mut h, o, v)
        }
        h[84..88].copy_from_slice(b"DXT1");
        h
    }
    #[test]
    fn skips_large_levels_and_rewrites_dimensions() {
        let p = make(&header(), 2872, 2).unwrap();
        assert_eq!(word(&p.header, 16), Some(16));
        assert_eq!(word(&p.header, 28), Some(5));
        assert_eq!(p.pixels.start, 128 + 2048 + 512);
        assert!(p.pixels.len() < 512);
    }
    #[test]
    fn unsupported_and_truncated_inputs_bypass() {
        assert!(make(&header(), 128, 2).is_none());
        let mut h = header();
        write(&mut h, 112, 0x200);
        assert!(make(&h, 4000, 2).is_none());
        h[84..88].copy_from_slice(b"DX10");
        assert!(make(&h, 4000, 2).is_none());
    }
    #[test]
    fn group_and_quality_rules() {
        assert_eq!(levels(b"foo.dds", b"Landscape", 4), 4);
        assert_eq!(levels(b"foo_LO.dds", b"General", 4), 2);
        assert_eq!(levels(b"foo_HI.dds", b"General", 4), 4);
        assert_eq!(levels(b"foo.dds", b"GUI", 4), 0);
        assert_eq!(levels(b"foo.dds", b"PLSM2", 4), 0);
        assert_eq!(levels(b"foo.dds", b"Landscape", 0), 0);
    }
}
