use std::time::Instant;

pub struct Timer {
    last: Instant,
    pub dt: f32,
    pub fps: f32,
    frame_count: u32,
    acc: f32,
}

impl Timer {
    pub fn new() -> Self {
        Self { last: Instant::now(), dt: 0.0, fps: 0.0, frame_count: 0, acc: 0.0 }
    }

    pub fn tick(&mut self) {
        let now = Instant::now();
        self.dt  = (now - self.last).as_secs_f32().min(0.05);
        self.last = now;
        self.acc += self.dt;
        self.frame_count += 1;
        if self.acc >= 1.0 {
            self.fps = self.frame_count as f32 / self.acc;
            self.frame_count = 0;
            self.acc = 0.0;
        }
    }
}
