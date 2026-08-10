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
use glam::Vec3;
use anyhow::Result;
use std::time::{Duration, Instant};

use core::{Config, Timer};
use world::World;
use render::{Renderer, Camera};
use player::{Player, Input};

struct DebugInfo {
    frame_count: u64,
    last_print:  Instant,
    fps:         f32,
}

impl DebugInfo {
    fn new() -> Self {
        Self { frame_count: 0, last_print: Instant::now(), fps: 0.0 }
    }

    fn tick(&mut self, player: &Player, world: &World) {
        self.frame_count += 1;
        let elapsed = self.last_print.elapsed();
        if elapsed < Duration::from_secs(1) { return; }

        self.fps         = self.frame_count as f32 / elapsed.as_secs_f32();
        self.frame_count = 0;
        self.last_print  = Instant::now();

        let pos       = player.pos;
        let vel       = player.velocity;
        let yaw_deg   = player.yaw.to_degrees();
        let pitch_deg = player.pitch.to_degrees();
        let chunks    = world.chunks.len();
        let gnd       = if player.on_ground { "GND" } else { "AIR" };
        // RaycastHit fields: pos, prev, normal, distance  (no 'block' field)
        let hit_str = match &player.hit {
            Some(h) => format!("[{},{},{}] dist={:.2}", h.pos.x, h.pos.y, h.pos.z, h.distance),
            None    => "none".into(),
        };

        eprintln!(
            "\x1b[36m[DBG]\x1b[0m \
             FPS={fps:5.1} | \
             XYZ=({x:7.2},{y:6.2},{z:7.2}) | \
             vel=({vx:5.2},{vy:5.2},{vz:5.2}) | {gnd} | \
             yaw={yaw:6.1}deg pitch={pit:5.1}deg | \
             chunks={ch:4} | hit={hit}",
            fps=self.fps,
            x=pos.x, y=pos.y, z=pos.z,
            vx=vel.x, vy=vel.y, vz=vel.z,
            gnd=gnd, yaw=yaw_deg, pit=pitch_deg,
            ch=chunks, hit=hit_str,
        );
    }
}

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

    fn grab_cursor(w: &Window) {
        if w.set_cursor_grab(CursorGrabMode::Locked).is_err() {
            let _ = w.set_cursor_grab(CursorGrabMode::Confined);
        }
        w.set_cursor_visible(false);
    }
    fn release_cursor(w: &Window) {
        let _ = w.set_cursor_grab(CursorGrabMode::None);
        w.set_cursor_visible(true);
    }
}

impl ApplicationHandler for Game {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title("Hurno -- Minecraft in Rust")
            .with_inner_size(winit::dpi::PhysicalSize::new(
                self.cfg.window_width, self.cfg.window_height,
            ));
        let win = std::sync::Arc::new(event_loop.create_window(attrs).unwrap());
        let win_ref: &'static Window = unsafe { &*(std::sync::Arc::as_ptr(&win)) };
        let renderer = pollster::block_on(Renderer::new(win_ref)).unwrap();
        self.renderer = Some(renderer);
        self.window   = Some(win);
        self.world.load_around(0, 0);

        eprintln!();
        eprintln!("\x1b[32m+------------------------------------------+\x1b[0m");
        eprintln!("\x1b[32m|  HURNO -- Minecraft Classic in Rust      |\x1b[0m");
        eprintln!("\x1b[32m+------------------------------------------+\x1b[0m");
        eprintln!("  WASD=move  Space=jump  Shift=sprint");
        eprintln!("  LMB=lock/break  RMB=place  Scroll=hotbar");
        eprintln!("  Esc=release mouse   F3=instant snapshot");
        eprintln!("\x1b[32m+------------------------------------------+\x1b[0m");
        eprintln!("  Seed        : {}",   self.cfg.seed);
        eprintln!("  Render dist : {} chunks", self.cfg.render_distance);
        eprintln!("  Chunks init : {}",   self.world.chunks.len());
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
                eprintln!("[HURNO] lost focus -- mouse released");
            }

            WindowEvent::Resized(sz) => {
                if let Some(r) = &mut self.renderer {
                    r.resize(sz.width, sz.height);
                    self.camera.resize(sz.width, sz.height);
                }
                eprintln!("[HURNO] resized {}x{}", sz.width, sz.height);
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
                            eprintln!("[HURNO] Esc -- mouse unlocked");
                        }
                        KeyCode::F3 if pressed => {
                            let p  = self.player.pos;
                            let cx = p.x as i32 >> 4;
                            let cz = p.z as i32 >> 4;
                            eprintln!();
                            eprintln!("\x1b[33m=== F3 SNAPSHOT ===\x1b[0m");
                            eprintln!("  Position  : ({:.2}, {:.2}, {:.2})", p.x, p.y, p.z);
                            eprintln!("  Chunk XZ  : ({}, {})", cx, cz);
                            eprintln!("  Yaw       : {:.1} deg", self.player.yaw.to_degrees());
                            eprintln!("  Pitch     : {:.1} deg", self.player.pitch.to_degrees());
                            eprintln!("  Velocity  : ({:.2}, {:.2}, {:.2})",
                                self.player.velocity.x,
                                self.player.velocity.y,
                                self.player.velocity.z);
                            eprintln!("  On ground : {}", self.player.on_ground);
                            eprintln!("  Chunks    : {}", self.world.chunks.len());
                            match &self.player.hit {
                                Some(h) => eprintln!(
                                    "  Hit block : [{},{},{}] dist={:.2} normal=({},{},{})",
                                    h.pos.x, h.pos.y, h.pos.z, h.distance,
                                    h.normal.x, h.normal.y, h.normal.z),
                                None => eprintln!("  Hit block : none"),
                            }
                            eprintln!("  Hotbar    : slot {}", self.player.inventory.selected);
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
                            eprintln!("[HURNO] mouse locked -- Esc to release");
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

                self.input.mouse_dx    = 0.0;
                self.input.mouse_dy    = 0.0;
                self.input.scroll      = 0;
                self.input.break_block = false;
                self.input.place_block = false;

                let pos = self.player.pos;
                let cx  = (pos.x as i32) >> 4;
                let cz  = (pos.z as i32) >> 4;
                self.world.load_around(cx, cz);
                self.world.unload_far(cx, cz);

                self.dbg.tick(&self.player, &self.world);

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
                                        load:  wgpu::LoadOp::Clear(wgpu::Color {
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
                            // TODO: bind chunk render pipeline & draw VBOs
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

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
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
