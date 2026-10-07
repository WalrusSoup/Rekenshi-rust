//! Own a read-only map while preventing ordinary Windows writers/truncation.
use super::tiff::Layout;
use memmap2::Mmap;
use std::fs::{File, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;

pub(super) struct Heightmap {
    map: Mmap,
    pub layout: Layout,
    _file: File,
}
impl Heightmap {
    pub fn open(path: &Path) -> Result<Self, String> {
        let file = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(path)
            .map_err(|e| format!("open heightmap read-only: {e}"))?;
        // SAFETY: This Windows handle denies write/delete sharing, including
        // incompatible preexisting writers. It stays open until after map drop.
        // The game opens the heightmap read-only; this adapter never modifies it.
        let map = unsafe { Mmap::map(&file) }.map_err(|e| e.to_string())?;
        let layout = Layout::parse(&map)?;
        Ok(Self {
            map,
            layout,
            _file: file,
        })
    }
    pub fn row(&self, x: usize, y: usize, width: usize) -> Option<&[u8]> {
        self.layout.row(&self.map, x, y, width)
    }
}
