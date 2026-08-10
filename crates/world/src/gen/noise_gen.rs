use noise::{NoiseFn, Perlin, Fbm};
use crate::{
    block::BlockId,
    chunk::{Chunk, CHUNK_W, CHUNK_D, CHUNK_H},
};

pub struct WorldGen {
    fbm:    Fbm<Perlin>,
    cave:   Perlin,
    seed:   u64,
}

impl WorldGen {
    pub fn new(seed: u64) -> Self {
        use noise::MultiFractal;
        let mut fbm = Fbm::<Perlin>::new(seed as u32);
        fbm = fbm.set_octaves(6).set_persistence(0.5).set_lacunarity(2.0);
        Self {
            fbm,
            cave: Perlin::new((seed >> 32) as u32),
            seed,
        }
    }

    pub fn fill(&self, chunk: &mut Chunk) {
        let cx = chunk.cx as f64;
        let cz = chunk.cz as f64;
        let sea_level: usize = 64;

        for x in 0..CHUNK_W {
            for z in 0..CHUNK_D {
                let wx = cx * 16.0 + x as f64;
                let wz = cz * 16.0 + z as f64;

                // Terrain height via fBm
                let h = self.fbm.get([wx / 200.0, wz / 200.0]);
                let height = ((h * 0.5 + 0.5) * 40.0 + 60.0) as usize;
                let surface = height.min(CHUNK_H - 1);

                for y in 0..CHUNK_H {
                    let id = self.block_at(wx, y as f64, wz, y, surface, sea_level);
                    if id != BlockId::Air {
                        chunk.set(x, y, z, id);
                    }
                }
            }
        }
        // Plant trees
        self.plant_trees(chunk, sea_level);
    }

    fn block_at(&self, wx: f64, wy: f64, wz: f64, y: usize, surface: usize, sea_level: usize) -> BlockId {
        if y == 0 { return BlockId::Bedrock; }
        if y > surface {
            return if y <= sea_level { BlockId::Water } else { BlockId::Air };
        }

        // Cave carving
        let cave_val = self.cave.get([wx / 20.0, wy / 10.0, wz / 20.0]);
        if cave_val > 0.6 && y > 5 && y < surface { return BlockId::Air; }

        if y == surface {
            if surface <= sea_level + 1 { BlockId::Sand }
            else { BlockId::Grass }
        } else if y >= surface.saturating_sub(3) {
            if surface <= sea_level + 1 { BlockId::Sand } else { BlockId::Dirt }
        } else if y < 16 {
            // Ore distribution
            let ore = self.cave.get([wx / 5.0 + 100.0, wy / 5.0, wz / 5.0 + 200.0]);
            if ore > 0.75      { BlockId::CoalOre }
            else if ore > 0.85 { BlockId::IronOre }
            else if ore > 0.92 { BlockId::GoldOre }
            else               { BlockId::Stone }
        } else {
            let ore = self.cave.get([wx / 5.0 + 100.0, wy / 5.0, wz / 5.0 + 200.0]);
            if ore > 0.80 { BlockId::CoalOre }
            else if ore > 0.88 { BlockId::IronOre }
            else               { BlockId::Stone }
        }
    }

    fn plant_trees(&self, chunk: &mut Chunk, sea_level: usize) {
        use fastrand::Rng;
        let mut rng = Rng::with_seed(
            (chunk.cx as u64).wrapping_mul(0x517cc1b727220a95)
                ^ (chunk.cz as u64).wrapping_mul(0x6c62272e07bb0142)
                ^ self.seed
        );
        let count = rng.usize(0..4);
        for _ in 0..count {
            let tx = rng.usize(2..CHUNK_W - 2);
            let tz = rng.usize(2..CHUNK_D - 2);
            let sy = chunk.surface_y(tx, tz);
            if sy < sea_level + 2 || sy > 120 { continue; }
            if chunk.get(tx, sy, tz) != BlockId::Grass { continue; }
            let trunk_h = rng.usize(4..7);
            for dy in 1..=trunk_h {
                if sy + dy < 256 { chunk.set(tx, sy + dy, tz, BlockId::Log); }
            }
            // Leaves
            let top = sy + trunk_h;
            for dx in -2i32..=2 {
                for dz in -2i32..=2 {
                    for dy in -1i32..=2 {
                        let ly = (top as i32 + dy) as usize;
                        let lx = tx as i32 + dx;
                        let lz = tz as i32 + dz;
                        if lx < 0 || lx >= 16 || lz < 0 || lz >= 16 || ly >= 256 { continue; }
                        if dx == 0 && dz == 0 && dy <= 0 { continue; }
                        if chunk.get(lx as usize, ly, lz as usize) == BlockId::Air {
                            chunk.set(lx as usize, ly, lz as usize, BlockId::Leaves);
                        }
                    }
                }
            }
        }
    }
}
