//! wgpu Renderer — initialises device, surface, depth buffer, and render pipeline.

use wgpu::{
    Backends, Device, DeviceDescriptor, Extent3d, Features, Instance,
    InstanceDescriptor, Limits, PresentMode, Queue, RequestAdapterOptions,
    Surface, SurfaceConfiguration, SurfaceTexture, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor, PowerPreference,
};
use winit::window::Window;
use anyhow::Result;

pub struct Renderer {
    pub surface:       Surface<'static>,
    pub device:        Device,
    pub queue:         Queue,
    pub config:        SurfaceConfiguration,
    pub depth_texture: Texture,
    pub depth_view:    TextureView,
}

impl Renderer {
    pub async fn new(window: &'static Window) -> Result<Self> {
        let size = window.inner_size();

        // ✅ Instance::new takes InstanceDescriptor by value (no &)
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference:       PowerPreference::HighPerformance,
                compatible_surface:     Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("no suitable GPU adapter found");

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label:             Some("hurno-device"),
                    required_features: Features::empty(),
                    required_limits:   Limits::default(),
                    ..Default::default()
                },
                None,
            )
            .await?;

        let caps   = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let config = SurfaceConfiguration {
            usage:        TextureUsages::RENDER_ATTACHMENT,
            format,
            width:        size.width.max(1),
            height:       size.height.max(1),
            present_mode: PresentMode::AutoVsync,
            alpha_mode:   caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let (depth_texture, depth_view) = Self::create_depth(&device, &config);

        Ok(Self { surface, device, queue, config, depth_texture, depth_view })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 { return; }
        self.config.width  = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        let (dt, dv) = Self::create_depth(&self.device, &self.config);
        self.depth_texture = dt;
        self.depth_view    = dv;
    }

    fn create_depth(device: &Device, config: &SurfaceConfiguration) -> (Texture, TextureView) {
        let tex = device.create_texture(&TextureDescriptor {
            label: Some("depth-texture"),
            size: Extent3d {
                width:                 config.width,
                height:                config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count:    1,
            dimension:       TextureDimension::D2,
            format:          TextureFormat::Depth32Float,
            usage:           TextureUsages::RENDER_ATTACHMENT,
            view_formats:    &[],
        });
        let view = tex.create_view(&TextureViewDescriptor::default());
        (tex, view)
    }

    /// Acquire the next frame. Returns `None` if the surface is lost.
    pub fn begin_frame(&self) -> Option<(SurfaceTexture, TextureView)> {
        let output = self.surface.get_current_texture().ok()?;
        let view   = output.texture.create_view(&TextureViewDescriptor::default());
        Some((output, view))
    }
}
