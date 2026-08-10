use bytemuck::{Pod, Zeroable};
use world::{Chunk, World, block_def, BlockId, BlockFace, CHUNK_W, CHUNK_H, CHUNK_D};
use glam::IVec2;
use crate::texture_atlas::tile_uv;

/// One vertex in the chunk mesh
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ChunkVertex {
    pub pos:    [f32; 3],
    pub uv:     [f32; 2],
    pub normal: [f32; 3],
    pub ao:     f32,
}

pub struct ChunkMesh {
    pub vertices: Vec<ChunkVertex>,
    pub indices:  Vec<u32>,
}

impl ChunkMesh {
    pub fn new() -> Self { Self { vertices: Vec::new(), indices: Vec::new() } }
}

/// Build the full greedy mesh for a chunk (simple face-by-face for now)
pub fn build_chunk_mesh(chunk: &Chunk, world: &World) -> ChunkMesh {
    let mut mesh = ChunkMesh::new();
    let ox = (chunk.cx * 16) as f32;
    let oz = (chunk.cz * 16) as f32;

    for x in 0..CHUNK_W {
        for y in 0..CHUNK_H {
            for z in 0..CHUNK_D {
                let id = chunk.get(x, y, z);
                if id == BlockId::Air { continue; }
                let def = block_def(id);

                let wx = chunk.cx * 16 + x as i32;
                let wy = y as i32;
                let wz = chunk.cz * 16 + z as i32;

                let faces = [
                    (BlockFace::Top,    [0,1,0],  [[0,0,1],[1,0,1],[1,0,0],[0,0,0]]),
                    (BlockFace::Bottom, [0,-1,0], [[0,1,0],[1,1,0],[1,1,1],[0,1,1]]),
                    (BlockFace::North,  [0,0,-1], [[0,0,0],[1,0,0],[1,1,0],[0,1,0]]),
                    (BlockFace::South,  [0,0,1],  [[1,0,1],[0,0,1],[0,1,1],[1,1,1]]),
                    (BlockFace::East,   [1,0,0],  [[1,0,0],[1,0,1],[1,1,1],[1,1,0]]),
                    (BlockFace::West,   [-1,0,0], [[0,0,1],[0,0,0],[0,1,0],[0,1,1]]),
                ];

                for (face, norm, corners) in faces {
                    let nx = wx + norm[0];
                    let ny = wy + norm[1];
                    let nz = wz + norm[2];
                    let neighbour = world.get_block(nx, ny, nz);
                    let n_def = block_def(neighbour);
                    if neighbour != BlockId::Air && !n_def.transparent { continue; }
                    if neighbour == id && def.transparent { continue; }

                    let uv = tile_uv(def.tex_for(face));
                    let uv_corners = [[uv[0],uv[3]],[uv[2],uv[3]],[uv[2],uv[1]],[uv[0],uv[1]]];
                    let norm_f = [norm[0] as f32, norm[1] as f32, norm[2] as f32];

                    let base = mesh.vertices.len() as u32;
                    for (i, c) in corners.iter().enumerate() {
                        mesh.vertices.push(ChunkVertex {
                            pos: [
                                ox + x as f32 + c[0] as f32,
                                y  as f32      + c[1] as f32,
                                oz + z as f32 + c[2] as f32,
                            ],
                            uv: uv_corners[i],
                            normal: norm_f,
                            ao: 1.0,
                        });
                    }
                    mesh.indices.extend_from_slice(&[base,base+1,base+2, base,base+2,base+3]);
                }
            }
        }
    }
    mesh
}
