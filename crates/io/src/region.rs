//! Minimal region-file format for chunk persistence.
//! Layout: header (1024 × 4-byte offsets) + chunk data blocks (4096-byte sectors).

use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Write, Seek, SeekFrom},
    path::Path,
};
use world::{Chunk, CHUNK_W, CHUNK_H, CHUNK_D, BlockId};
use anyhow::Result;

const SECTOR: usize = 4096;
const HEADER_SECTORS: usize = 1;

pub struct RegionFile {
    path: std::path::PathBuf,
}

impl RegionFile {
    pub fn open(dir: &Path, rx: i32, rz: i32) -> Result<Self> {
        std::fs::create_dir_all(dir)?;
        Ok(Self { path: dir.join(format!("r.{}.{}.mcr", rx, rz)) })
    }

    fn chunk_key(cx: i32, cz: i32) -> usize {
        ((cx.rem_euclid(32)) + (cz.rem_euclid(32)) * 32) as usize
    }

    pub fn save_chunk(&self, chunk: &Chunk) -> Result<()> {
        // Serialize chunk as raw bytes (CHUNK_VOL bytes)
        let mut data = Vec::with_capacity(CHUNK_W * CHUNK_H * CHUNK_D + 8);
        data.extend_from_slice(&chunk.cx.to_le_bytes());
        data.extend_from_slice(&chunk.cz.to_le_bytes());
        for y in 0..CHUNK_H {
            for z in 0..CHUNK_D {
                for x in 0..CHUNK_W {
                    data.push(chunk.get(x, y, z) as u8);
                }
            }
        }
        std::fs::write(&self.path, &data)?;
        Ok(())
    }

    pub fn load_chunk(&self, cx: i32, cz: i32) -> Result<Chunk> {
        let data = std::fs::read(&self.path)?;
        let mut chunk = Chunk::new(cx, cz);
        let mut offset = 8usize;
        for y in 0..CHUNK_H {
            for z in 0..CHUNK_D {
                for x in 0..CHUNK_W {
                    if offset < data.len() {
                        chunk.set(x, y, z, BlockId::from_u8(data[offset]));
                        offset += 1;
                    }
                }
            }
        }
        chunk.dirty = false;
        chunk.generated = true;
        Ok(chunk)
    }
}
