// SPDX-License-Identifier: Unlicense

//! Window + render-surface lifecycle.
//!
//! winit owns the OS window + event loop. wgpu is the render
//! surface; we ship pixels onto it. Today we paint a placeholder
//! (clear color + status text) so the shell + automation layer
//! can be exercised before WebRender / Servo's paint stack is
//! wired in. Replacing the placeholder paint with the real engine
//! is one method (`Engine::paint(&mut surface)`) that gets routed
//! to whichever Engine impl is active.

use std::sync::Arc;

use winit::dpi::LogicalSize;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

use glyphon::FontSystem;

use super::text::{page_runs, TextLayer, TextRun};

/// Per-window state the shell maintains.
pub struct WindowState {
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub clear_color: wgpu::Color,
    pub text: TextLayer,
    /// Current page text for rendering. Updated by the shell
    /// after navigation; rendered every frame.
    pub url_text: String,
    pub title_text: String,
    pub body_text: String,
}

impl WindowState {
    /// Construct the window, request a wgpu surface, and pick an
    /// adapter. Synchronous via `pollster`; this is one-time
    /// startup work.
    ///
    /// `font_system` is built by the caller (App::new) before the
    /// event loop starts, NOT inside this constructor. Building
    /// it here would freeze the winit `resumed` event handler for
    /// hundreds of ms while it scans system font directories
    /// (Rust audit bug #6). The caller hands ownership in.
    pub fn new(
        event_loop: &ActiveEventLoop,
        title: &str,
        font_system: FontSystem,
    ) -> anyhow::Result<Self> {
        let attributes = WindowAttributes::default()
            .with_title(title)
            .with_inner_size(LogicalSize::new(1280.0, 800.0));
        let window = Arc::new(event_loop.create_window(attributes)?);
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        // SAFETY: the Arc<Window> we hold lives as long as
        // WindowState, which lives as long as the surface.
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| anyhow::anyhow!("create_surface: {e}"))?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| anyhow::anyhow!("no compatible wgpu adapter found"))?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("atsisbroken-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let mut text = TextLayer::new(&device, &queue, surface_format, font_system);
        text.resize(&queue, size.width.max(1), size.height.max(1));

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            // Default to a near-black clear color matching the
            // TUI aesthetic. Each render pass overwrites this
            // before paint when the engine has content.
            clear_color: wgpu::Color {
                r: 0.07,
                g: 0.08,
                b: 0.10,
                a: 1.0,
            },
            text,
            url_text: String::new(),
            title_text: "atsisbroken".to_string(),
            body_text: "Loading…".to_string(),
        })
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.text.resize(&self.queue, new_size.width, new_size.height);
        }
    }

    /// Update the page text the next frame will render. Called by
    /// the shell after navigation completes.
    pub fn set_page(
        &mut self,
        url: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) {
        self.url_text = url.into();
        self.title_text = title.into();
        self.body_text = body.into();
        self.window.request_redraw();
    }

    /// Render one frame. Clear pass + text overlay. When stylo +
    /// WebRender are wired, the clear pass becomes the first of
    /// many; layout-driven paint commands stack on top.
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Build the per-frame text run list. When the shell asks
        // for an internal page like atsisbroken://connections, the
        // body is whatever HTML-parsed text the engine produced.
        // For now, body is set explicitly via set_page.
        let runs: Vec<TextRun> = page_runs(
            &self.url_text,
            &self.title_text,
            &self.body_text,
            self.config.width,
            self.config.height,
        );

        // glyphon prepare must happen BEFORE the render pass
        // begins (it uses the device + queue to upload glyphs to
        // its atlas). Errors here bubble up — typically only fail
        // on out-of-memory in the atlas.
        if let Err(e) = self.text.prepare(&self.device, &self.queue, &runs) {
            eprintln!("text prepare: {e}");
        }

        let mut encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("atsisbroken-encoder"),
                });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear-and-text-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            // Render the text into the same pass as the clear so
            // the glyphs land directly on the cleared background.
            if let Err(e) = self.text.render(&mut rpass) {
                eprintln!("text render: {e}");
            }
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}
