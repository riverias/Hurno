use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub window_width:  u32,
    pub window_height: u32,
    pub render_distance: i32,
    pub fov_deg: f32,
    pub mouse_sensitivity: f32,
    pub seed: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            window_width:  1280,
            window_height: 720,
            render_distance: 8,
            fov_deg: 70.0,
            mouse_sensitivity: 0.15,
            seed: 12345,
        }
    }
}

impl Config {
    pub fn load_or_default(path: &str) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }
}
