use std::collections::HashMap;
use glam::IVec2;
use crate::{
    chunk::Chunk,
    block::BlockId,
    gen::WorldGen,
};

pub struct World {
    pub chunks: HashMap<IVec2, Chunk>,
    gen: WorldGen,
    pub render_distance: i32,
}

impl World {
    pub fn new(seed: u64, render_distance: i32) -> Self {
        Self {
            chunks: HashMap::new(),
            gen: WorldGen::new(seed),
            render_distance,
        }
    }

    /// Ensure all chunks in render_distance around (cx,cz) are loaded
    pub fn load_around(&mut self, cx: i32, cz: i32) {
        let rd = self.render_distance;
        for dx in -rd..=rd {
            for dz in -rd..=rd {
                let key = IVec2::new(cx + dx, cz + dz);
                if !self.chunks.contains_key(&key) {
                    let mut chunk = Chunk::new(key.x, key.y);
                    self.gen.fill(&mut chunk);
                    chunk.generated = true;
                    self.chunks.insert(key, chunk);
                }
            }
        }
    }

    pub fn get_block(&self, wx: i32, wy: i32, wz: i32) -> BlockId {
        if wy < 0 || wy >= 256 { return BlockId::Air; }
        let key = IVec2::new(wx >> 4, wz >> 4);
        self.chunks.get(&key).map(|c| {
            c.get(
                (wx.rem_euclid(16)) as usize,
                wy as usize,
                (wz.rem_euclid(16)) as usize,
            )
        }).unwrap_or(BlockId::Air)
    }

    pub fn set_block(&mut self, wx: i32, wy: i32, wz: i32, id: BlockId) {
        if wy < 0 || wy >= 256 { return; }
        let key = IVec2::new(wx >> 4, wz >> 4);
        if let Some(chunk) = self.chunks.get_mut(&key) {
            chunk.set(
                (wx.rem_euclid(16)) as usize,
                wy as usize,
                (wz.rem_euclid(16)) as usize,
                id,
            );
            // also mark neighbours dirty if on edge
            let lx = wx.rem_euclid(16);
            let lz = wz.rem_euclid(16);
            let neighbours = [
                (lx == 0,  IVec2::new(key.x - 1, key.y)),
                (lx == 15, IVec2::new(key.x + 1, key.y)),
                (lz == 0,  IVec2::new(key.x, key.y - 1)),
                (lz == 15, IVec2::new(key.x, key.y + 1)),
            ];
            for (cond, nk) in neighbours {
                if cond {
                    if let Some(nc) = self.chunks.get_mut(&nk) {
                        nc.dirty = true;
                    }
                }
            }
        }
    }

    /// Unload chunks too far from the player
    pub fn unload_far(&mut self, cx: i32, cz: i32) {
        let rd = self.render_distance + 2;
        self.chunks.retain(|k, _| {
            (k.x - cx).abs() <= rd && (k.y - cz).abs() <= rd
        });
    }
}
