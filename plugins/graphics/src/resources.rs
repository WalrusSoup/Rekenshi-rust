//! Copied resource descriptions and logical payload estimates, never RAM/VRAM accounting.
#![forbid(unsafe_code)]
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct Description {
    pub kind: &'static str,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub array_size: u32,
    pub mip_levels: u32,
    pub samples: u32,
    pub format: i32,
    pub usage: i32,
    pub bind_flags: u32,
    pub cpu_access_flags: u32,
    pub misc_flags: u32,
    pub buffer_bytes: u32,
    pub structure_stride: u32,
    pub initial_data: bool,
}
impl Description {
    /// Tight logical payload only: excludes tiling, alignment, metadata and CPU copies.
    pub fn estimated_bytes(&self) -> Option<u64> {
        if self.kind == "buffer" {
            return Some(u64::from(self.buffer_bytes));
        }
        if [
            self.width,
            self.height,
            self.depth,
            self.array_size,
            self.samples,
        ]
        .contains(&0)
        {
            return None;
        }
        let (block_width, block_height, block_bytes) = match self.format {
            1..=4 => (1, 1, 16),
            5..=8 => (1, 1, 12),
            9..=22 | 102 => (1, 1, 8),
            23..=47 | 67 | 87..=93 | 100..=101 => (1, 1, 4),
            48..=59 | 85..=86 | 115 => (1, 1, 2),
            60..=65 => (1, 1, 1),
            66 => (8, 1, 1),
            68..=69 => (2, 1, 4),
            70..=72 | 79..=81 => (4, 4, 8),
            73..=78 | 82..=84 | 94..=99 => (4, 4, 16),
            // Planar/video/newer formats remain explicitly unknown.
            _ => return None,
        };
        let maximum = self.width.max(self.height).max(self.depth);
        let full_chain = u32::BITS - maximum.leading_zeros();
        let levels = if self.mip_levels == 0 {
            full_chain
        } else {
            self.mip_levels
        };
        if levels > full_chain {
            return None;
        }
        let mut bytes = 0u64;
        for level in 0..levels {
            let w = u64::from((self.width >> level).max(1)).div_ceil(block_width);
            let h = u64::from((self.height >> level).max(1)).div_ceil(block_height);
            let d = u64::from((self.depth >> level).max(1));
            bytes =
                bytes.checked_add(w.checked_mul(h)?.checked_mul(d)?.checked_mul(block_bytes)?)?;
        }
        bytes
            .checked_mul(u64::from(self.array_size))?
            .checked_mul(u64::from(self.samples))
    }
}

#[cfg(test)]
mod tests {
    use super::Description;
    fn texture(format: i32, width: u32, height: u32) -> Description {
        Description {
            kind: "texture2d",
            width,
            height,
            depth: 1,
            array_size: 1,
            mip_levels: 1,
            samples: 1,
            format,
            usage: 0,
            bind_flags: 0,
            cpu_access_flags: 0,
            misc_flags: 0,
            buffer_bytes: 0,
            structure_stride: 0,
            initial_data: false,
        }
    }
    #[test]
    fn estimates_compressed_edges_mips_arrays_and_samples() {
        assert_eq!(
            texture(28, 4096, 4096).estimated_bytes(),
            Some(64 * 1024 * 1024)
        );
        assert_eq!(texture(71, 5, 7).estimated_bytes(), Some(32));
        assert_eq!(texture(98, 5, 7).estimated_bytes(), Some(64));
        let mut d = texture(28, 4, 4);
        d.mip_levels = 0;
        d.array_size = 6;
        assert_eq!(d.estimated_bytes(), Some((64 + 16 + 4) * 6));
        d.mip_levels = 1;
        d.array_size = 1;
        d.samples = 4;
        assert_eq!(d.estimated_bytes(), Some(256));
        d.kind = "texture3d";
        d.samples = 1;
        d.depth = 4;
        d.mip_levels = 0;
        assert_eq!(d.estimated_bytes(), Some(256 + 32 + 4));
    }
    #[test]
    fn unknown_formats_invalid_mips_and_overflow_are_not_reported_as_zero() {
        assert_eq!(texture(103, 4, 4).estimated_bytes(), None);
        let mut d = texture(28, 4, 4);
        d.mip_levels = 20;
        assert_eq!(d.estimated_bytes(), None);
        d = texture(2, u32::MAX, u32::MAX);
        assert_eq!(d.estimated_bytes(), None);
        d.kind = "buffer";
        d.buffer_bytes = 4096;
        assert_eq!(d.estimated_bytes(), Some(4096));
    }
}
