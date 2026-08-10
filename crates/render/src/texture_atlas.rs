use image::RgbaImage;

pub const ATLAS_COLS: u32 = 16;
pub const ATLAS_ROWS: u32 = 16;
pub const TILE_PX:   u32 = 16;

/// Returns UV rectangle [u0, v0, u1, v1] for atlas tile index `idx`
pub fn tile_uv(idx: u8) -> [f32; 4] {
    let col = (idx as u32 % ATLAS_COLS) as f32;
    let row = (idx as u32 / ATLAS_COLS) as f32;
    let s   = 1.0 / ATLAS_COLS as f32;
    let t   = 1.0 / ATLAS_ROWS as f32;
    // slight inset to avoid bleeding
    let eps = 0.001;
    [col * s + eps, row * t + eps, (col + 1.0) * s - eps, (row + 1.0) * t - eps]
}

pub fn load_atlas(bytes: &[u8]) -> RgbaImage {
    image::load_from_memory(bytes)
        .expect("failed to decode terrain.png")
        .into_rgba8()
}
