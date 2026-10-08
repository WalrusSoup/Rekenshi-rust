//! Own a read-only map while preventing ordinary Windows writers/truncation.
use super::tiff::Layout;
use memmap2::Mmap;
use std::fs::{File, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::Arc;

pub(super) struct Heightmap {
    map: Arc<Mmap>,
    eager: Option<Vec<u8>>,
    pub layout: Layout,
    _file: Arc<File>,
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
            map: Arc::new(map),
            eager: None,
            layout,
            _file: Arc::new(file),
        })
    }
    pub fn row(&self, x: usize, y: usize, width: usize) -> Option<&[u8]> {
        let bytes = self.eager.as_deref().unwrap_or(&self.map);
        self.layout.row(bytes, x, y, width)
    }

    /// Make a complete owned copy before publishing it to native readers.
    /// This commits memory but does not pin physical pages against OS paging.
    pub fn eager_copy(&self) -> Result<Self, String> {
        // Bound extra commit even for a valid TIFF with large unused padding.
        if self.map.len() > 1024 * 1024 * 1024 {
            return Err("Gigacache supports heightmap files up to 1 GiB".into());
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(self.map.len())
            .map_err(|e| format!("allocate Gigacache: {e}"))?;
        bytes.extend_from_slice(&self.map);
        let layout = Layout::parse(&bytes)?;
        Ok(Self {
            map: Arc::clone(&self.map),
            eager: Some(bytes),
            layout,
            _file: Arc::clone(&self._file),
        })
    }

    pub fn eager_bytes(&self) -> usize {
        self.eager.as_ref().map_or(0, Vec::len)
    }
}
