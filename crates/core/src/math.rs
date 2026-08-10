use glam::{Vec3, IVec3};

/// World-space position → chunk coordinates
#[inline]
pub fn world_to_chunk(pos: Vec3) -> IVec3 {
    IVec3::new(
        pos.x.floor() as i32 >> 4,
        0,
        pos.z.floor() as i32 >> 4,
    )
}

/// World-space position → block-local coords within chunk
#[inline]
pub fn world_to_local(pos: Vec3) -> (usize, usize, usize) {
    (
        (pos.x.floor() as i32).rem_euclid(16) as usize,
        pos.y.floor() as usize,
        (pos.z.floor() as i32).rem_euclid(16) as usize,
    )
}
