use winit::{
    application::ApplicationHandler,
    event::{
        DeviceEvent, DeviceId, ElementState, KeyEvent,
        MouseButton, MouseScrollDelta, WindowEvent,
    },
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};
use glam::{IVec2, Vec3};
use anyhow::Result;
use std::time::{Duration, Instant};

use core::{Config, Timer};
use world::World;
use render::{Camera, ChunkRenderer, Renderer};
use player::{Player, Input};

// ---- Debug ------------------------------------------------------------------
struct Dbg { frames: u64, last: Instant, fps: f32 }
impl Dbg {
    fn new() -> Self { Self { frames: 0, last: Instant::now(), fps: 0.0 } }
    fn tick(&mut self, p: &Player, w: &World, cr: &ChunkRenderer) {
        self.frames += 1;
        let e = self.last.elapsed();
        if e < Duration::from_secs(1) { return; }
        self.fps    = self.frames as f32 / e.as_secs_f32();
        self.frames = 0;
        self.last   = Instant::now();
        let pos = p.pos;
        let vel = p.velocity;
        let hit = match &p.hit {
            Some(h) => format!("[{},{},{}] d={:.1}", h.pos.x, h.pos.y, h.pos.z, h.distance),
            None    => "none".into(),
        };
        eprintln!(
            "\x1b[36m[DBG]\x1b[0m FPS={fps:5.1} | \
             XYZ=({x:7.2},{y:6.2},{z:7.2}) | vel=({vx:5.2},{vy:5.2},{vz:5.2}) | {gnd} | \
             yaw={yaw:6.1} pitch={pit:5.1} | chunks={ch:4}/{gpu} | hit={hit}",
            fps = self.fps,
            x=pos.x, y=pos.y, z=pos.z,
            vx=vel.x, vy=vel.y, vz=vel.z,
            gnd = if p.on_ground { "GND" } else { "AIR" },
            yaw = p.yaw.to_degrees(), pit = p.pitch.to_degrees(),
            ch  = w.chunks.len(), gpu = cr.gpu_chunks.len(),
            hit = hit,
        );
    }
}

// ---- Game -------------------------------------------------------------------
struct Game {
    window:        Option<std::sync::Arc<Window>>,
    renderer:      Option<Renderer>,
    chunk_render:  Option<ChunkRenderer>,
    world:         World,
    camera:        Camera,
    player:        Player,
    timer:         Timer,
    input:         Input,
    cfg:           Config,
    mouse_locked:  bool,
    dbg:           Dbg,
}

impl Game {
    fn new(cfg: Config) -> Self {
        let world  = World::new(cfg.seed, cfg.render_distance);
        let camera = Camera::new(cfg.fov_deg, cfg.window_width, cfg.window_height);
        let player = Player::new(Vec3::new(0.0, 80.0, 0.0));
        Self {
            window: None, renderer: None, chunk_render: None,
            world, camera, player,
            timer: Timer::new(), input: Input::default(),
            cfg, mouse_locked: false, dbg: Dbg::new(),
        }
    }
    fn grab(w: &Window) {
        if w.set_cursor_grab(CursorGrabMode::Locked).is_err() {
            let _ = w.set_cursor_grab(CursorGrabMode::Confined);
        }
        w.set_cursor_visible(false);
    }
    fn release(w: &Window) {
        let _ = w.set_cursor_grab(CursorGrabMode::None);
        w.set_cursor_visible(true);
    }
}

// ---- ApplicationHandler -----------------------------------------------------
impl ApplicationHandler for Game {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title("Hurno -- Minecraft in Rust")
            .with_inner_size(winit::dpi::PhysicalSize::new(
                self.cfg.window_width, self.cfg.window_height));
        let win     = std::sync::Arc::new(el.create_window(attrs).unwrap());
        let win_ref: &'static Window = unsafe { &*(std::sync::Arc::as_ptr(&win)) };
        let renderer = pollster::block_on(Renderer::new(win_ref)).unwrap();

        // Build ChunkRenderer using the surface format from the renderer
        let mut cr = ChunkRenderer::new(
            &renderer.device, &renderer.queue, renderer.config.format);

        // Initial chunk load + first mesh upload
        self.world.load_around(0, 0);
        let keys: Vec<IVec2> = self.world.chunks.keys().cloned().collect();
        eprintln!("[HURNO] uploading {} chunks to GPU...", keys.len());
        for key in &keys {
            cr.upload_chunk(&renderer.device, *key, &self.world);
        }
        // Clear dirty flags
        for key in &keys {
            if let Some(c) = self.world.chunks.get_mut(key) { c.dirty = false; }
        }
        eprintln!("[HURNO] {} chunk meshes uploaded", cr.gpu_chunks.len());

        self.renderer     = Some(renderer);
        self.chunk_render = Some(cr);
        self.window       = Some(win);

        eprintln!();
        eprintln!("\x1b[32m+--------------------------------------+\x1b[0m");
        eprintln!("\x1b[32m| HURNO -- Minecraft Classic in Rust   |\x1b[0m");
        eprintln!("\x1b[32m+--------------------------------------+\x1b[0m");
        eprintln!("  WASD=move  Space=jump  Shift=sprint");
        eprintln!("  LMB=lock/break  RMB=place  Scroll=hotbar");
        eprintln!("  Esc=release mouse   F3=snapshot");
        eprintln!("  Seed: {}  RenderDist: {}", self.cfg.seed, self.cfg.render_distance);
        eprintln!();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),

            WindowEvent::Focused(false) => {
                self.mouse_locked = false;
                if let Some(w) = &self.window { Self::release(w); }
                eprintln!("[HURNO] lost focus");
            }

            WindowEvent::Resized(sz) => {
                if let Some(r) = &mut self.renderer {
                    r.resize(sz.width, sz.height);
                    self.camera.resize(sz.width, sz.height);
                }
            }

            WindowEvent::KeyboardInput {
                event: KeyEvent { physical_key, state, .. }, ..
            } => {
                let p = state == ElementState::Pressed;
                if let PhysicalKey::Code(key) = physical_key {
                    match key {
                        KeyCode::KeyW      => self.input.forward  = p,
                        KeyCode::KeyS      => self.input.backward = p,
                        KeyCode::KeyA      => self.input.left     = p,
                        KeyCode::KeyD      => self.input.right    = p,
                        KeyCode::Space     => self.input.jump     = p,
                        KeyCode::ShiftLeft => self.input.sprint   = p,
                        KeyCode::Escape if p => {
                            self.mouse_locked = false;
                            if let Some(w) = &self.window { Self::release(w); }
                            eprintln!("[HURNO] mouse unlocked");
                        }
                        KeyCode::F3 if p => {
                            let pos = self.player.pos;
                            eprintln!("\x1b[33m=== F3 ===\x1b[0m");
                            eprintln!("  XYZ     : ({:.2},{:.2},{:.2})", pos.x,pos.y,pos.z);
                            eprintln!("  Chunk   : ({},{})", pos.x as i32>>4, pos.z as i32>>4);
                            eprintln!("  Yaw/Pit : {:.1}/{:.1}",
                                      self.player.yaw.to_degrees(),
                                      self.player.pitch.to_degrees());
                            eprintln!("  Velocity: ({:.2},{:.2},{:.2})",
                                      self.player.velocity.x,
                                      self.player.velocity.y,
                                      self.player.velocity.z);
                            eprintln!("  OnGround: {}", self.player.on_ground);
                            let world_ch = self.world.chunks.len();
                            let gpu_ch   = self.chunk_render.as_ref().map(|r| r.gpu_chunks.len()).unwrap_or(0);
                            eprintln!("  Chunks  : {} world / {} gpu", world_ch, gpu_ch);
                            match &self.player.hit {
                                Some(h) => eprintln!("  Hit     : [{},{},{}] dist={:.2}",
                                    h.pos.x,h.pos.y,h.pos.z,h.distance),
                                None    => eprintln!("  Hit     : none"),
                            }
                            eprintln!("  Hotbar  : slot {}", self.player.inventory.selected);
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
                            if let Some(w) = &self.window { Self::grab(w); }
                            eprintln!("[HURNO] mouse locked -- Esc to release");
                        } else { self.input.break_block = true; }
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

                // --- game logic ---
                self.player.update(&mut self.world, &self.input, dt);
                let vp  = self.camera.view_proj(self.player.view_matrix());
                let eye = self.player.eye_pos();

                // reset transient inputs
                self.input.mouse_dx    = 0.0;
                self.input.mouse_dy    = 0.0;
                self.input.scroll      = 0;
                self.input.break_block = false;
                self.input.place_block = false;

                // --- chunk load / unload ---
                let cx = (self.player.pos.x as i32) >> 4;
                let cz = (self.player.pos.z as i32) >> 4;
                self.world.load_around(cx, cz);
                self.world.unload_far(cx, cz);

                // --- mesh upload: new & dirty chunks ---
                // collect keys to upload (immutable borrow of world)
                let to_upload: Vec<IVec2> = {
                    let cr_ref = self.chunk_render.as_ref();
                    self.world.chunks.iter()
                        .filter(|(k, c)| {
                            c.dirty || cr_ref.map(|r| !r.gpu_chunks.contains_key(k)).unwrap_or(true)
                        })
                        .map(|(k, _)| *k)
                        .collect()
                };

                if let (Some(cr), Some(r)) =
                    (&mut self.chunk_render, &self.renderer)
                {
                    // remove GPU chunks for unloaded world chunks
                    cr.gpu_chunks.retain(|k, _| self.world.chunks.contains_key(k));

                    // upload new / dirty chunks (limit per frame to avoid freezes)
                    let limit = 16_usize;
                    for &key in to_upload.iter().take(limit) {
                        cr.upload_chunk(&r.device, key, &self.world);
                    }

                    // update camera uniform
                    cr.update_camera(&r.queue, vp, eye);
                }

                // clear dirty flags after upload
                for &key in &to_upload {
                    if let Some(c) = self.world.chunks.get_mut(&key) { c.dirty = false; }
                }

                // --- debug line ---
                if let Some(cr) = &self.chunk_render {
                    self.dbg.tick(&self.player, &self.world, cr);
                }

                // --- GPU render ---
                let cr_ref = &self.chunk_render;
                if let Some(r) = &self.renderer {
                    if let Some((output, view)) = r.begin_frame() {
                        let mut enc = r.device.create_command_encoder(
                            &wgpu::CommandEncoderDescriptor { label: Some("frame") });
                        {
                            let mut rpass = enc.begin_render_pass(
                                &wgpu::RenderPassDescriptor {
                                    label: Some("main"),
                                    color_attachments: &[Some(
                                        wgpu::RenderPassColorAttachment {
                                            view: &view,
                                            resolve_target: None,
                                            ops: wgpu::Operations {
                                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                                    r: 0.53, g: 0.81, b: 0.98, a: 1.0,
                                                }),
                                                store: wgpu::StoreOp::Store,
                                            },
                                        }
                                    )],
                                    depth_stencil_attachment: Some(
                                        wgpu::RenderPassDepthStencilAttachment {
                                            view: &r.depth_view,
                                            depth_ops: Some(wgpu::Operations {
                                                load:  wgpu::LoadOp::Clear(1.0),
                                                store: wgpu::StoreOp::Store,
                                            }),
                                            stencil_ops: None,
                                        }
                                    ),
                                    ..Default::default()
                                }
                            );
                            // Draw all uploaded chunk meshes
                            if let Some(cr) = cr_ref {
                                cr.draw(&mut rpass);
                            }
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

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, ev: DeviceEvent) {
        if !self.mouse_locked { return; }
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = ev {
            self.input.mouse_dx += dx as f32;
            self.input.mouse_dy += dy as f32;
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(w) = &self.window { w.request_redraw(); }
    }
}

// ---- main -------------------------------------------------------------------
fn main() -> Result<()> {
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
