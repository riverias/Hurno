use glam::{Vec3, IVec3};
use world::{World, BlockId, block_def};

pub struct RaycastHit {
    pub pos:      IVec3,   // block that was hit
    pub prev:     IVec3,   // block position to place into
    pub normal:   IVec3,   // face normal
    pub distance: f32,
}

/// DDA raycast through the voxel world
pub fn raycast(world: &World, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<RaycastHit> {
    let dir = dir.normalize();
    let mut pos = IVec3::new(
        origin.x.floor() as i32,
        origin.y.floor() as i32,
        origin.z.floor() as i32,
    );
    let step   = IVec3::new(dir.x.signum() as i32, dir.y.signum() as i32, dir.z.signum() as i32);
    let t_delta = Vec3::new(
        (1.0 / dir.x).abs(),
        (1.0 / dir.y).abs(),
        (1.0 / dir.z).abs(),
    );
    let boundary = Vec3::new(
        (if step.x > 0 { pos.x + 1 } else { pos.x }) as f32,
        (if step.y > 0 { pos.y + 1 } else { pos.y }) as f32,
        (if step.z > 0 { pos.z + 1 } else { pos.z }) as f32,
    );
    let mut t_max = Vec3::new(
        (boundary.x - origin.x) / dir.x,
        (boundary.y - origin.y) / dir.y,
        (boundary.z - origin.z) / dir.z,
    );
    // Fix NaN for zero dir components
    if dir.x == 0.0 { t_max.x = f32::INFINITY; }
    if dir.y == 0.0 { t_max.y = f32::INFINITY; }
    if dir.z == 0.0 { t_max.z = f32::INFINITY; }

    let mut prev = pos;
    let mut dist = 0.0f32;

    while dist < max_dist {
        let id = world.get_block(pos.x, pos.y, pos.z);
        if id != BlockId::Air && block_def(id).solid {
            let axis_dir = pos - prev;
            return Some(RaycastHit {
                pos,
                prev,
                normal: IVec3::new(-axis_dir.x, -axis_dir.y, -axis_dir.z),
                distance: dist,
            });
        }
        prev = pos;
        if t_max.x < t_max.y && t_max.x < t_max.z {
            dist = t_max.x; t_max.x += t_delta.x; pos.x += step.x;
        } else if t_max.y < t_max.z {
            dist = t_max.y; t_max.y += t_delta.y; pos.y += step.y;
        } else {
            dist = t_max.z; t_max.z += t_delta.z; pos.z += step.z;
        }
    }
    None
}
