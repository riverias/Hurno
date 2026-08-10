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
    window:   Option<std::sync::Arc<Window>>,
    renderer: Option<Renderer>,
    world:    World,
    camera:   Camera,
    player:   Player,
    timer:    Timer,
    input:    Input,
    cfg:      Config,
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
}

impl ApplicationHandler for Game {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title("Hurno — Minecraft in Rust")
            .with_inner_size(winit::dpi::PhysicalSize::new(
                self.cfg.window_width, self.cfg.window_height));
        let win = std::sync::Arc::new(event_loop.create_window(attrs).unwrap());
        let win_ref: &'static Window = unsafe { &*(std::sync::Arc::as_ptr(&win)) };
        let renderer = pollster::block_on(Renderer::new(win_ref)).unwrap();
        self.renderer = Some(renderer);
        self.window   = Some(win);

        // Initial chunk load
        self.world.load_around(0, 0);
        info!("Game initialised");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer {
                    r.resize(size.width, size.height);
                    self.camera.resize(size.width, size.height);
                }
            }

            WindowEvent::KeyboardInput { event: KeyEvent { physical_key, state, .. }, .. } => {
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
                            if let Some(w) = &self.window {
                                let _ = w.set_cursor_grab(CursorGrabMode::None);
                                w.set_cursor_visible(true);
                            }
                        }
                        _ => {}
                    }
                }
            }

            WindowEvent::MouseInput { state: ElementState::Pressed, button, .. } => {
                match button {
                    MouseButton::Left  => {
                        if !self.mouse_locked {
                            self.mouse_locked = true;
                            if let Some(w) = &self.window {
                                let _ = w.set_cursor_grab(CursorGrabMode::Confined);
                                w.set_cursor_visible(false);
                            }
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
                    MouseButton::Left  => self.input.break_block  = false,
                    MouseButton::Right => self.input.place_block  = false,
                    _ => {}
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y as i32,
                    MouseScrollDelta::PixelDelta(p)   => -(p.y as i32).signum(),
                };
                self.input.scroll = scroll;
            }

            WindowEvent::RedrawRequested => {
                self.timer.tick();
                let dt = self.timer.dt;

                // Player update
                self.player.update(&mut self.world, &self.input, dt);

                // Reset per-frame inputs
                self.input.mouse_dx = 0.0;
                self.input.mouse_dy = 0.0;
                self.input.scroll   = 0;

                // Load chunks around player
                let cp = self.player.pos;
                self.world.load_around((cp.x as i32) >> 4, (cp.z as i32) >> 4);
                self.world.unload_far((cp.x as i32) >> 4, (cp.z as i32) >> 4);

                // Render
                if let Some(r) = &self.renderer {
                    if let Some((output, view)) = r.begin_frame() {
                        let mut encoder = r.device.create_command_encoder(
                            &wgpu::CommandEncoderDescriptor { label: Some("frame") });
                        {
                            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("main"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color { r:0.53, g:0.81, b:0.98, a:1.0 }),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                    view: &r.depth_view,
                                    depth_ops: Some(wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(1.0),
                                        store: wgpu::StoreOp::Store,
                                    }),
                                    stencil_ops: None,
                                }),
                                ..Default::default()
                            });
                            // TODO: draw chunk meshes here
                        }
                        r.queue.submit(std::iter::once(encoder.finish()));
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
        if let DeviceEvent::MouseMotion { delta } = event {
            self.input.mouse_dx += delta.0 as f32;
            self.input.mouse_dy += delta.1 as f32;
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(w) = &self.window { w.request_redraw(); }
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let cfg = Config::load_or_default("config.toml");
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut game = Game::new(cfg);
    event_loop.run_app(&mut game)?;
    Ok(())
}
