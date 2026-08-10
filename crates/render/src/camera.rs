use glam::{Mat4, Vec3};

pub struct Camera {
    pub fov_y:  f32,
    pub aspect: f32,
    pub near:   f32,
    pub far:    f32,
}

impl Camera {
    pub fn new(fov_deg: f32, width: u32, height: u32) -> Self {
        Self {
            fov_y:  fov_deg.to_radians(),
            aspect: width as f32 / height as f32,
            near:   0.05,
            far:    1000.0,
        }
    }

    pub fn proj_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, self.aspect, self.near, self.far)
    }

    pub fn view_proj(&self, view: Mat4) -> Mat4 {
        self.proj_matrix() * view
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }
}
