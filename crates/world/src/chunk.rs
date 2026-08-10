use crate::block::BlockId;

pub const CHUNK_W: usize = 16;
pub const CHUNK_H: usize = 256;
pub const CHUNK_D: usize = 16;
pub const CHUNK_VOL: usize = CHUNK_W * CHUNK_H * CHUNK_D;

/// A 16×256×16 column of blocks
#[derive(Clone)]
pub struct Chunk {
    pub cx: i32,
    pub cz: i32,
    data: Box<[u8; CHUNK_VOL]>,   // stored as u8 to save memory
    pub dirty: bool,               // needs remesh
    pub generated: bool,
}

impl Chunk {
    pub fn new(cx: i32, cz: i32) -> Self {
        Self {
            cx,
            cz,
            data: Box::new([0u8; CHUNK_VOL]),
            dirty: true,
            generated: false,
        }
    }

    #[inline(always)]
    pub fn idx(x: usize, y: usize, z: usize) -> usize {
        // Y-major layout for cache-friendly vertical iteration
        y * (CHUNK_W * CHUNK_D) + z * CHUNK_W + x
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> BlockId {
        BlockId::from_u8(self.data[Self::idx(x, y, z)])
    }

    #[inline]
    pub fn get_raw(&self, x: usize, y: usize, z: usize) -> u8 {
        self.data[Self::idx(x, y, z)]
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, id: BlockId) {
        self.data[Self::idx(x, y, z)] = id as u8;
        self.dirty = true;
    }

    /// Height of the highest non-air block in this column
    pub fn surface_y(&self, x: usize, z: usize) -> usize {
        for y in (0..CHUNK_H).rev() {
            if self.get(x, y, z) != BlockId::Air { return y; }
        }
        0
    }
}
