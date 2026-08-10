// ─── Hurno – Minecraft in Rust ───────────────────────────────────────────────
// Debug console: every second the terminal prints a status line with
//   FPS | XYZ position | yaw/pitch | loaded chunks | targeted block
// Controls: WASD = move  Space = jump  Shift = sprint
//           Left-click = lock mouse / break block
//           Right-click = place block   Scroll = hotbar
//           Esc = release mouse
// ─────────────────────────────────────────────────────────────────────────────

use winit::{
    application::ApplicationHandler,
    event::{
        WindowEvent, DeviceEvent, DeviceId,
        ElementState, MouseButton, MouseScrollDelta,
        KeyEvent,
    },
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId, CursorGrabMode},
};
use glam::Vec3;
use anyhow::Result;
use std::time::{Duration, Instant};

use core::{Config, Timer};
use world::World;
use render::{Renderer, Camera};
use player::{Player, Input};

// ── Debug state ──────────────────────────────────────────────────────────────
struct DebugInfo {
    frame_count:    u64,
    last_print:     Instant,
    fps:            f32,
}

impl DebugInfo {
    fn new() -> Self {
        Self { frame_count: 0, last_print: Instant::now(), fps: 0.0 }
    }

    /// Call once per frame. Returns true when a new debug line was printed.
    fn tick(&mut self, player: &Player, world: &World) -> bool {
        self.frame_count += 1;
        let elapsed = self.last_print.elapsed();
        if elapsed >= Duration::from_secs(1) {
            self.fps       = self.frame_count as f32 / elapsed.as_secs_f32();
            self.frame_count = 0;
            self.last_print  = Instant::now();

            let pos        = player.pos;
            let yaw_deg    = player.yaw.to_degrees();
            let pitch_deg  = player.pitch.to_degrees();
            let chunks     = world.chunks.len();
            let hit_str    = match &player.hit {
                Some(h) => format!("[{},{},{}] {:?}", h.pos.x, h.pos.y, h.pos.z, h.block),
                None    => "none".to_string(),
            };
            let on_gnd     = if player.on_ground { "GND" } else { "AIR" };
            let vel        = player.velocity;

            // ── print to stderr so it's always visible even with log filters ──
            eprintln!(
                "\x1b[36m[HURNO DBG]\x1b[0m \
                 FPS={fps:5.1} | \
                 XYZ=({x:7.2},{y:6.2},{z:7.2}) | \
                 vel=({vx:5.2},{vy:5.2},{vz:5.2}) | {gnd} | \
                 yaw={yaw:6.1}° pitch={pit:5.1}° | \
                 chunks={chunks:4} | \
                 hit={hit}",
                fps    = self.fps,
                x      = pos.x,  y = pos.y,  z = pos.z,
                vx     = vel.x,  vy = vel.y, vz = vel.z,
                gnd    = on_gnd,
                yaw    = yaw_deg,
                pit    = pitch_deg,
                chunks = chunks,
                hit    = hit_str,
            );
            return true;
        }
        false
    }
}

// ── Game struct ───────────────────────────────────────────────────────────────
struct Game {
    window:       Option<std::sync::Arc<Window>>,
    renderer:     Option<Renderer>,
    world:        World,
    camera:       Camera,
    player:       Player,
    timer:        Timer,
    input:        Input,
    cfg:          Config,
    mouse_locked: bool,
    dbg:          DebugInfo,
}

impl Game {
    fn new(cfg: Config) -> Self {
        let world  = World::new(cfg.seed, cfg.render_distance);
        let camera = Camera::new(cfg.fov_deg, cfg.window_width, cfg.window_height);
        let player = Player::new(Vec3::new(0.0, 80.0, 0.0));
        Self {
            window: None, renderer: None,
            world, camera, player,
            timer: Timer::new(),
            input: Input::default(),
            cfg,
            mouse_locked: false,
            dbg: DebugInfo::new(),
        }
    }

    fn grab_cursor(window: &Window) {
        if window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
            let _ = window.set_cursor_grab(CursorGrabMode::Confined);
        }
        window.set_cursor_visible(false);
    }

    fn release_cursor(window: &Window) {
        let _ = window.set_cursor_grab(CursorGrabMode::None);
        window.set_cursor_visible(true);
    }
}

// ── winit ApplicationHandler ──────────────────────────────────────────────────
impl ApplicationHandler for Game {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title("Hurno \u2014 Minecraft in Rust")
            .with_inner_size(winit::dpi::PhysicalSize::new(
                self.cfg.window_width, self.cfg.window_height));
        let win = std::sync::Arc::new(event_loop.create_window(attrs).unwrap());
        let win_ref: &'static Window = unsafe { &*(std::sync::Arc::as_ptr(&win)) };
        let renderer = pollster::block_on(Renderer::new(win_ref)).unwrap();
        self.renderer = Some(renderer);
        self.window   = Some(win);
        self.world.load_around(0, 0);

        // ── startup banner ────────────────────────────────────────────────
        eprintln!();
        eprintln!("\x1b[32m╔══════════════════════════════════════════════╗\x1b[0m");
        eprintln!("\x1b[32m║       HURNO — Minecraft Classic in Rust      ║\x1b[0m");
        eprintln!("\x1b[32m╠══════════════════════════════════════════════╣\x1b[0m");
        eprintln!("\x1b[32m║\x1b[0m  WASD = move     Space = jump  Shift = sprint  \x1b[32m║\x1b[0m");
        eprintln!("\x1b[32m║\x1b[0m  LMB  = lock mouse / break block               \x1b[32m║\x1b[0m");
        eprintln!("\x1b[32m║\x1b[0m  RMB  = place block   Scroll = hotbar          \x1b[32m║\x1b[0m");
        eprintln!("\x1b[32m║\x1b[0m  Esc  = release mouse                          \x1b[32m║\x1b[0m");
        eprintln!("\x1b[32m╠══════════════════════════════════════════════╣\x1b[0m");
        eprintln!("\x1b[32m║\x1b[0m  [HURNO DBG] line prints every second          \x1b[32m║\x1b[0m");
        eprintln!("\x1b[32m╚══════════════════════════════════════════════╝\x1b[0m");
        eprintln!();
        eprintln!("  Seed       : {}", self.cfg.seed);
        eprintln!("  Render dist: {} chunks", self.cfg.render_distance);
        eprintln!("  Chunks loaded at start: {}", self.world.chunks.len());
        eprintln!();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Focused(false) => {
                self.mouse_locked = false;
                if let Some(w) = &self.window { Self::release_cursor(w); }
                eprintln!("[HURNO] window lost focus \u2014 mouse released");
            }

            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer {
                    r.resize(size.width, size.height);
                    self.camera.resize(size.width, size.height);
                }
                eprintln!("[HURNO] resized to {}x{}", size.width, size.height);
            }

            WindowEvent::KeyboardInput {
                event: KeyEvent { physical_key, state, .. }, ..
            } => {
                let pressed = state == ElementState::Pressed;
                if let PhysicalKey::Code(key) = physical_key {
                    match key {
                        KeyCode::KeyW      => self.input.forward  = pressed,
                        KeyCode::KeyS      => self.input.backward = pressed,
                        KeyCode::KeyA      => self.input.left     = pressed,
                        KeyCode::KeyD      => self.input.right    = pressed,
                        KeyCode::Space     => self.input.jump     = pressed,
                        KeyCode::ShiftLeft => self.input.sprint   = pressed,
                        KeyCode::Escape if pressed => {
                            self.mouse_locked = false;
                            if let Some(w) = &self.window { Self::release_cursor(w); }
                            eprintln!("[HURNO] Esc \u2014 mouse unlocked");
                        }
                        // F3 \u2014 print instant debug snapshot
                        KeyCode::F3 if pressed => {
                            let pos   = self.player.pos;
                            let chunk = (pos.x as i32 >> 4, pos.z as i32 >> 4);
                            eprintln!();
                            eprintln!("\x1b[33m[F3 SNAPSHOT]\x1b[0m");
                            eprintln!("  Position  : ({:.2}, {:.2}, {:.2})", pos.x, pos.y, pos.z);
                            eprintln!("  Chunk XZ  : {:?}", chunk);
                            eprintln!("  Yaw/Pitch : {:.1}\u00b0 / {:.1}\u00b0",
                                      self.player.yaw.to_degrees(),
                                      self.player.pitch.to_degrees());
                            eprintln!("  Velocity  : ({:.2}, {:.2}, {:.2})",
                                      self.player.velocity.x,
                                      self.player.velocity.y,
                                      self.player.velocity.z);
                            eprintln!("  On ground : {}", self.player.on_ground);
                            eprintln!("  Chunks    : {}", self.world.chunks.len());
                            eprintln!("  Block hit : {:?}", self.player.hit);
                            eprintln!("  Hotbar sel: {}", self.player.inventory.selected);
                            eprintln!();
                        }
                        _ => {}
                    }
                }
            }

            WindowEvent::MouseInput { state: ElementState::Pressed, button, .. } => {
                match button {
                    MouseButton::Left => {
                        if !self.mouse_locked {
                            self.mouse_locked = true;
                            if let Some(w) = &self.window { Self::grab_cursor(w); }
                            eprintln!("[HURNO] mouse locked \u2014 Esc to release");
                        } else {
                            self.input.break_block = true;
                        }
                    }
                    MouseButton::Right => self.input.place_block = true,
                    _ => {}
                }
            }
            WindowEvent::MouseInput { state: ElementState::Released, button, .. } => {
                match button {
                    MouseButton::Left  => self.input.break_block = false,
                    MouseButton::Right => self.input.place_block = false,
                    _ => {}
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                self.input.scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y as i32,
                    MouseScrollDelta::PixelDelta(p)   => -(p.y as i32).signum(),
                };
            }

            WindowEvent::RedrawRequested => {
                self.timer.tick();
                let dt = self.timer.dt;

                self.player.update(&mut self.world, &self.input, dt);
                let _vp = self.camera.view_proj(self.player.view_matrix());

                // Per-frame transient reset
                self.input.mouse_dx    = 0.0;
                self.input.mouse_dy    = 0.0;
                self.input.scroll      = 0;
                self.input.break_block = false;
                self.input.place_block = false;

                // Chunk management
                let pos = self.player.pos;
                let cx  = (pos.x as i32) >> 4;
                let cz  = (pos.z as i32) >> 4;
                self.world.load_around(cx, cz);
                self.world.unload_far(cx, cz);

                // Debug console output (every 1 s)
                self.dbg.tick(&self.player, &self.world);

                // GPU render
                if let Some(r) = &self.renderer {
                    if let Some((output, view)) = r.begin_frame() {
                        let mut enc = r.device.create_command_encoder(
                            &wgpu::CommandEncoderDescriptor { label: Some("frame") });
                        {
                            let _rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("main-pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color {
                                            r: 0.53, g: 0.81, b: 0.98, a: 1.0,
                                        }),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: Some(
                                    wgpu::RenderPassDepthStencilAttachment {
                                        view: &r.depth_view,
                                        depth_ops: Some(wgpu::Operations {
                                            load:  wgpu::LoadOp::Clear(1.0),
                                            store: wgpu::StoreOp::Store,
                                        }),
                                        stencil_ops: None,
                                    },
                                ),
                                ..Default::default()
                            });
                            // TODO: bind chunk pipeline & draw mesh VBOs here
                        }
                        r.queue.submit(std::iter::once(enc.finish()));
                        output.present();
                    }
                }

                if let Some(w) = &self.window { w.request_redraw(); }
            }

            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _: &ActiveEventLoop,
        _: DeviceId,
        event: DeviceEvent,
    ) {
        if !self.mouse_locked { return; }
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event {
            self.input.mouse_dx += dx as f32;
            self.input.mouse_dy += dy as f32;
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(w) = &self.window { w.request_redraw(); }
    }
}

// ── main ─────────────────────────────────────────────────────────────────────
fn main() -> Result<()> {
    // Suppress wgpu_core noise; keep our own app=info logs.
    // Set RUST_LOG=debug in env to see everything.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,app=info"))
        )
        .init();

    let cfg        = Config::load_or_default("config.toml");
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut game   = Game::new(cfg);
    event_loop.run_app(&mut game)?;
    Ok(())
}
