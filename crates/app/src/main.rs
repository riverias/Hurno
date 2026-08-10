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
use tracing::info;

use core::{Config, Timer};
use world::World;
use render::{Renderer, Camera};
use player::{Player, Input};

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
        }
    }

    /// Try Locked (true FPS — cursor stays hidden & centred), fall back to Confined.
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

impl ApplicationHandler for Game {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title("Hurno — Minecraft in Rust")
            .with_inner_size(winit::dpi::PhysicalSize::new(
                self.cfg.window_width, self.cfg.window_height));
        let win = std::sync::Arc::new(event_loop.create_window(attrs).unwrap());
        // SAFETY: window lives for the entire process lifetime inside self.window
        let win_ref: &'static Window = unsafe { &*(std::sync::Arc::as_ptr(&win)) };
        let renderer = pollster::block_on(Renderer::new(win_ref)).unwrap();
        self.renderer = Some(renderer);
        self.window   = Some(win);
        self.world.load_around(0, 0);
        info!("Game ready — left-click to lock mouse, Esc to release");
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            // ── lifecycle ─────────────────────────────────────────────────────
            WindowEvent::CloseRequested => event_loop.exit(),

            // Release grab when alt-tabbing away
            WindowEvent::Focused(false) => {
                self.mouse_locked = false;
                if let Some(w) = &self.window { Self::release_cursor(w); }
            }

            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer {
                    r.resize(size.width, size.height);
                    self.camera.resize(size.width, size.height);
                }
            }

            // ── keyboard ──────────────────────────────────────────────────────
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
                        }
                        _ => {}
                    }
                }
            }

            // ── mouse buttons ─────────────────────────────────────────────────
            WindowEvent::MouseInput { state: ElementState::Pressed, button, .. } => {
                match button {
                    MouseButton::Left => {
                        if !self.mouse_locked {
                            // First click: grab mouse for FPS look
                            self.mouse_locked = true;
                            if let Some(w) = &self.window { Self::grab_cursor(w); }
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

            // ── scroll (hotbar) ───────────────────────────────────────────────
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y as i32,
                    MouseScrollDelta::PixelDelta(p)   => -(p.y as i32).signum(),
                };
            }

            // ── frame ─────────────────────────────────────────────────────────
            WindowEvent::RedrawRequested => {
                self.timer.tick();
                let dt = self.timer.dt;

                // Update player (mouse_dx/dy → yaw/pitch inside player.update)
                self.player.update(&mut self.world, &self.input, dt);

                // view-projection (available for pipeline uniform later)
                let _vp = self.camera.view_proj(self.player.view_matrix());

                // Reset per-frame transient inputs
                self.input.mouse_dx    = 0.0;
                self.input.mouse_dy    = 0.0;
                self.input.scroll      = 0;
                self.input.break_block = false;
                self.input.place_block = false;

                // Dynamic chunk loading
                let pos = self.player.pos;
                let cx  = (pos.x as i32) >> 4;
                let cz  = (pos.z as i32) >> 4;
                self.world.load_around(cx, cz);
                self.world.unload_far(cx, cz);

                // GPU render
                if let Some(r) = &self.renderer {
                    if let Some((output, view)) = r.begin_frame() {
                        let mut enc = r.device.create_command_encoder(
                            &wgpu::CommandEncoderDescriptor { label: Some("frame") });
                        {
                            // Sky-blue clear; chunk draw calls go inside once pipeline is wired
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

    /// Raw device events — used for FPS mouse look (works even with Confined mode).
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

fn main() -> Result<()> {
    // Suppress the wgpu_core "waiting for submission" INFO spam.
    // Override with RUST_LOG=debug to see everything.
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
