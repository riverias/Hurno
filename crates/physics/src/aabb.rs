use glam::Vec3;

/// Axis-Aligned Bounding Box
#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(center: Vec3, half_extents: Vec3) -> Self {
        Self { min: center - half_extents, max: center + half_extents }
    }

    #[inline]
    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.x < other.max.x && self.max.x > other.min.x &&
        self.min.y < other.max.y && self.max.y > other.min.y &&
        self.min.z < other.max.z && self.max.z > other.min.z
    }

    pub fn translate(&self, delta: Vec3) -> Self {
        Self { min: self.min + delta, max: self.max + delta }
    }

    /// Sweep self along `velocity`, return the t [0,1] of first block hit
    pub fn sweep(&self, velocity: Vec3, other: &Aabb) -> Option<f32> {
        if velocity == Vec3::ZERO { return None; }
        let mut t_enter = 0.0f32;
        let mut t_exit  = 1.0f32;
        for axis in 0..3 {
            let v = velocity[axis];
            let (s_min, s_max, o_min, o_max) = (
                self.min[axis], self.max[axis],
                other.min[axis], other.max[axis],
            );
            if v.abs() < 1e-6 {
                if s_max <= o_min || s_min >= o_max { return None; }
            } else {
                let t1 = (o_min - s_max) / v;
                let t2 = (o_max - s_min) / v;
                let (t1, t2) = if t1 < t2 { (t1, t2) } else { (t2, t1) };
                t_enter = t_enter.max(t1);
                t_exit  = t_exit.min(t2);
                if t_enter > t_exit { return None; }
            }
        }
        if t_enter >= 0.0 && t_enter <= 1.0 { Some(t_enter) } else { None }
    }
}
