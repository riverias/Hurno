use player::{Inventory, HOTBAR_SIZE};

/// Hotbar / crosshair HUD — produces quads for the GUI renderer
#[derive(Default)]
pub struct Hud {
    pub quads: Vec<GuiQuad>,
}

#[derive(Debug, Clone, Copy)]
pub struct GuiQuad {
    pub x: f32, pub y: f32,
    pub w: f32, pub h: f32,
    pub u0: f32, pub v0: f32,
    pub u1: f32, pub v1: f32,
}

const SLOT_SIZE:    f32 = 40.0;
const SLOT_PADDING: f32 = 4.0;

impl Hud {
    pub fn build(&mut self, inv: &Inventory, screen_w: f32, screen_h: f32) {
        self.quads.clear();
        let total_w = HOTBAR_SIZE as f32 * (SLOT_SIZE + SLOT_PADDING) - SLOT_PADDING;
        let hx = (screen_w - total_w) * 0.5;
        let hy = screen_h - SLOT_SIZE - 8.0;
        // Hotbar slots (UV from GUI atlas — approximate)
        for i in 0..HOTBAR_SIZE {
            let sx = hx + i as f32 * (SLOT_SIZE + SLOT_PADDING);
            self.quads.push(GuiQuad { x: sx, y: hy, w: SLOT_SIZE, h: SLOT_SIZE,
                u0: 0.0, v0: 0.0, u1: 0.125, v1: 0.125 });
        }
        // Crosshair
        let cx = screen_w * 0.5 - 8.0;
        let cy = screen_h * 0.5 - 8.0;
        self.quads.push(GuiQuad { x: cx, y: cy, w: 16.0, h: 16.0,
            u0: 0.0, v0: 0.0, u1: 1.0, v1: 1.0 });
    }
}
