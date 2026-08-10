use glam::{Vec3, Mat4};
use world::{World, BlockId, block_def};
use physics::{Aabb, raycast, RaycastHit};
use crate::inventory::Inventory;

const PLAYER_HEIGHT: f32 = 1.8;
const PLAYER_WIDTH:  f32 = 0.6;
const EYE_OFFSET:    f32 = 1.62;
const GRAVITY:       f32 = -28.0;
const JUMP_SPEED:    f32 = 8.0;
const WALK_SPEED:    f32 = 4.3;
const SPRINT_SPEED:  f32 = 5.6;
const REACH:         f32 = 5.0;

#[derive(Debug, Clone, Copy, Default)]
pub struct Input {
    pub forward:  bool,
    pub backward: bool,
    pub left:     bool,
    pub right:    bool,
    pub jump:     bool,
    pub sprint:   bool,
    pub break_block:  bool,
    pub place_block:  bool,
    pub mouse_dx: f32,
    pub mouse_dy: f32,
    pub scroll:   i32,
}

pub struct Player {
    pub pos:       Vec3,      // feet position
    pub velocity:  Vec3,
    pub yaw:       f32,       // radians
    pub pitch:     f32,
    pub on_ground: bool,
    pub inventory: Inventory,
    pub sensitivity: f32,
    pub hit:       Option<RaycastHit>,
}

impl Player {
    pub fn new(spawn: Vec3) -> Self {
        Self {
            pos:       spawn,
            velocity:  Vec3::ZERO,
            yaw:       0.0,
            pitch:     0.0,
            on_ground: false,
            inventory: Inventory::classic_starter(),
            sensitivity: 0.0015,
            hit:       None,
        }
    }

    pub fn eye_pos(&self) -> Vec3 { self.pos + Vec3::Y * EYE_OFFSET }

    pub fn look_dir(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            -self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        ).normalize()
    }

    pub fn aabb(&self) -> Aabb {
        Aabb::new(
            self.pos + Vec3::new(0.0, PLAYER_HEIGHT / 2.0, 0.0),
            Vec3::new(PLAYER_WIDTH / 2.0, PLAYER_HEIGHT / 2.0, PLAYER_WIDTH / 2.0),
        )
    }

    pub fn update(&mut self, world: &mut World, input: &Input, dt: f32) {
        // Mouse look
        self.yaw   += input.mouse_dx * self.sensitivity;
        self.pitch  = (self.pitch + input.mouse_dy * self.sensitivity)
            .clamp(-std::f32::consts::FRAC_PI_2 + 0.01, std::f32::consts::FRAC_PI_2 - 0.01);

        // Scroll hotbar
        if input.scroll != 0 { self.inventory.scroll(input.scroll); }

        // Movement
        let speed = if input.sprint { SPRINT_SPEED } else { WALK_SPEED };
        let forward = Vec3::new(self.yaw.sin(), 0.0, self.yaw.cos());
        let right   = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        let mut move_dir = Vec3::ZERO;
        if input.forward  { move_dir += forward; }
        if input.backward { move_dir -= forward; }
        if input.right    { move_dir += right; }
        if input.left     { move_dir -= right; }
        if move_dir.length_squared() > 0.0 {
            move_dir = move_dir.normalize() * speed;
        }
        self.velocity.x = move_dir.x;
        self.velocity.z = move_dir.z;

        // Jump
        if input.jump && self.on_ground {
            self.velocity.y = JUMP_SPEED;
            self.on_ground  = false;
        }

        // Gravity
        self.velocity.y += GRAVITY * dt;
        self.velocity.y  = self.velocity.y.max(-50.0);

        // Move & collide
        self.move_and_collide(world, dt);

        // Raycast
        self.hit = raycast(world, self.eye_pos(), self.look_dir(), REACH);

        // Break / place
        if input.break_block {
            if let Some(ref h) = self.hit {
                world.set_block(h.pos.x, h.pos.y, h.pos.z, BlockId::Air);
            }
        }
        if input.place_block {
            if let Some(ref h) = self.hit {
                if let Some(id) = self.inventory.selected_block() {
                    let p = h.prev;
                    let pa = self.aabb();
                    let target_aabb = Aabb::new(
                        Vec3::new(p.x as f32 + 0.5, p.y as f32 + 0.5, p.z as f32 + 0.5),
                        Vec3::splat(0.5),
                    );
                    if !pa.intersects(&target_aabb) {
                        world.set_block(p.x, p.y, p.z, id);
                    }
                }
            }
        }
    }

    fn move_and_collide(&mut self, world: &World, dt: f32) {
        let mut delta = self.velocity * dt;
        // Resolve each axis independently
        for axis in 0..3 {
            let mut d = Vec3::ZERO;
            d[axis] = delta[axis];
            let next = self.pos + d;
            if !self.collides_at(world, next) {
                self.pos[axis] += delta[axis];
            } else {
                if axis == 1 {
                    self.on_ground = delta[1] < 0.0;
                }
                self.velocity[axis] = 0.0;
                delta[axis] = 0.0;
            }
        }
    }

    fn collides_at(&self, world: &World, pos: Vec3) -> bool {
        let half_w = PLAYER_WIDTH / 2.0;
        let mins = [
            (pos.x - half_w).floor() as i32,
            pos.y.floor() as i32,
            (pos.z - half_w).floor() as i32,
        ];
        let maxs = [
            (pos.x + half_w).ceil() as i32,
            (pos.y + PLAYER_HEIGHT).ceil() as i32,
            (pos.z + half_w).ceil() as i32,
        ];
        for bx in mins[0]..maxs[0] {
            for by in mins[1]..maxs[1] {
                for bz in mins[2]..maxs[2] {
                    let id = world.get_block(bx, by, bz);
                    if block_def(id).solid {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn view_matrix(&self) -> Mat4 {
        let eye = self.eye_pos();
        let dir = self.look_dir();
        Mat4::look_to_rh(eye, dir, Vec3::Y)
    }
}
