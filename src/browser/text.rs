// SPDX-License-Identifier: Unlicense
// Unlicense — public domain — cochranblock.org

//! Text rendering on the wgpu surface.
//!
//! Glyphon wraps cosmic-text (Unicode-correct shaping with font
//! fallback) and renders glyphs via a wgpu texture atlas. This
//! module is the bridge between "engine produced parsed HTML and
//! resolved layout" and "user sees pixels."
//!
//! Today's surface is intentionally thin. The browser shell hands
//! us three blocks of text (URL bar, page title, page body) at
//! fixed positions; we render them onto the wgpu pass. When stylo
//! and the layout engine land, the layout engine produces a list
//! of `(x, y, text, style)` tuples and this module renders that
//! same list. The interface stays stable; the producer changes.

#![cfg(feature = "gui")]

use glyphon::{
    Attrs, Buffer, Cache, Color, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};

/// One run of text to render at a fixed position. The shell builds
/// a Vec of these per frame; the renderer paints them in order
/// (later runs paint over earlier — last-wins on overlap).
pub struct TextRun {
    /// Logical pixel position of the run's top-left corner.
    pub x: f32,
    pub y: f32,
    /// Pixel-precise clip bounds. Glyphs outside this rect get
    /// clipped — used to keep long text from running off the page.
    pub width: u32,
    pub height: u32,
    /// Font size in logical pixels.
    pub size: f32,
    /// Line height; typical ratio is 1.4× font size.
    pub line_height: f32,
    /// RGB color in 0..=255.
    pub color: (u8, u8, u8),
    /// The text itself.
    pub text: String,
    /// Logical font family — sans-serif / monospace / serif.
    pub family: TextFamily,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFamily {
    SansSerif,
    Monospace,
    Serif,
}

/// Per-window text rendering state. Owns the glyphon resources
/// (font system, glyph atlas, renderer) and a buffer pool keyed
/// by run index. Re-used across frames; cosmic-text's shaping
/// cache makes repeated identical text render-time near-zero.
pub struct TextLayer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    cache: Cache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
    /// Buffer pool — grows as needed, never shrinks. Each run
    /// claims a buffer by index; if the pool is too small we
    /// allocate more on the fly. cosmic-text is robust to
    /// re-using buffers across frames with different text.
    buffers: Vec<Buffer>,
}

impl TextLayer {
    /// Construct the text layer with a pre-built FontSystem.
    /// FontSystem::new() runs a disk scan over the OS font dirs;
    /// previous code called it inside this constructor (and thus
    /// inside winit's `resumed` event handler), which froze the
    /// event loop for hundreds of ms on machines with many fonts
    /// (Rust audit bug #6). The caller now builds the FontSystem
    /// before the event loop starts and hands ownership in here.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        mut font_system: FontSystem,
    ) -> Self {
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, surface_format);
        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        // Pre-warm with one buffer so the first frame doesn't pay
        // the allocation cost. Empty buffer; gets overwritten on
        // first set_runs.
        let buffer = Buffer::new(&mut font_system, Metrics::new(16.0, 22.0));

        Self {
            font_system,
            swash_cache,
            cache,
            viewport,
            atlas,
            renderer,
            buffers: vec![buffer],
        }
    }

    /// Resize the viewport when the window resizes. Required so
    /// glyphon knows the surface dimensions for clip planes.
    pub fn resize(&mut self, queue: &wgpu::Queue, width: u32, height: u32) {
        self.viewport.update(
            queue,
            Resolution {
                width,
                height,
            },
        );
    }

    /// Build the per-frame TextArea list from the runs and ask
    /// glyphon to prepare them for rendering. Call before any
    /// render pass that wants to draw text.
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        runs: &[TextRun],
    ) -> Result<(), glyphon::PrepareError> {
        // Grow the buffer pool as needed.
        while self.buffers.len() < runs.len() {
            let buf = Buffer::new(
                &mut self.font_system,
                Metrics::new(16.0, 22.0),
            );
            self.buffers.push(buf);
        }
        // Update each buffer with this frame's text.
        for (i, run) in runs.iter().enumerate() {
            let buf = &mut self.buffers[i];
            buf.set_size(
                &mut self.font_system,
                Some(run.width as f32),
                Some(run.height as f32),
            );
            // Set metrics — font size + line height.
            buf.set_metrics(
                &mut self.font_system,
                Metrics::new(run.size, run.line_height),
            );
            let attrs = match run.family {
                TextFamily::SansSerif => Attrs::new().family(Family::SansSerif),
                TextFamily::Monospace => Attrs::new().family(Family::Monospace),
                TextFamily::Serif => Attrs::new().family(Family::Serif),
            };
            buf.set_text(
                &mut self.font_system,
                &run.text,
                attrs,
                Shaping::Advanced,
            );
            buf.shape_until_scroll(&mut self.font_system, false);
        }

        // Build TextAreas referencing the prepared buffers.
        let areas: Vec<TextArea> = runs
            .iter()
            .enumerate()
            .map(|(i, run)| TextArea {
                buffer: &self.buffers[i],
                left: run.x,
                top: run.y,
                scale: 1.0,
                bounds: TextBounds {
                    left: run.x as i32,
                    top: run.y as i32,
                    right: (run.x + run.width as f32) as i32,
                    bottom: (run.y + run.height as f32) as i32,
                },
                default_color: Color::rgb(run.color.0, run.color.1, run.color.2),
                custom_glyphs: &[],
            })
            .collect();

        self.renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            areas,
            &mut self.swash_cache,
        )
    }

    /// Render the prepared runs into the supplied render pass.
    /// Call after `prepare` and inside an active wgpu render pass.
    pub fn render<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
    ) -> Result<(), glyphon::RenderError> {
        self.renderer.render(&self.atlas, &self.viewport, pass)
    }

    /// Trim the glyph atlas — call between very different page
    /// loads to keep memory bounded. cosmic-text caches everything
    /// shaped; for a long browser session the cache can grow.
    pub fn trim_atlas(&mut self) {
        self.atlas.trim();
    }
}

/// Build the default page-render run-list from the three pieces
/// of text the shell has today: URL bar, page title, page body.
/// When stylo + layout land, this function gets replaced by the
/// layout engine's run-list output.
pub fn page_runs(
    url: &str,
    title: &str,
    body: &str,
    surface_width: u32,
    surface_height: u32,
) -> Vec<TextRun> {
    let margin = 24.0;
    let url_height = 32;
    let title_height = 56;
    let body_top = (margin + url_height as f32 + title_height as f32 + margin) as u32;
    let body_height = surface_height.saturating_sub(body_top + margin as u32);

    vec![
        // URL bar — top, monospace, dim.
        TextRun {
            x: margin,
            y: margin,
            width: surface_width.saturating_sub(margin as u32 * 2),
            height: url_height,
            size: 14.0,
            line_height: 18.0,
            color: (140, 150, 160),
            text: url.to_string(),
            family: TextFamily::Monospace,
        },
        // Page title — large, sans-serif, bright.
        TextRun {
            x: margin,
            y: margin + url_height as f32 + 8.0,
            width: surface_width.saturating_sub(margin as u32 * 2),
            height: title_height,
            size: 32.0,
            line_height: 40.0,
            color: (240, 240, 240),
            text: title.to_string(),
            family: TextFamily::SansSerif,
        },
        // Body — fill remaining space, sans-serif, normal.
        TextRun {
            x: margin,
            y: body_top as f32,
            width: surface_width.saturating_sub(margin as u32 * 2),
            height: body_height,
            size: 16.0,
            line_height: 22.0,
            color: (210, 215, 220),
            text: body.to_string(),
            family: TextFamily::SansSerif,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_runs_produce_three_blocks() {
        let runs = page_runs(
            "atsisbroken://home",
            "atsisbroken",
            "Connect what you've made.",
            1280,
            800,
        );
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].text, "atsisbroken://home");
        assert_eq!(runs[0].family, TextFamily::Monospace);
        assert_eq!(runs[1].text, "atsisbroken");
        assert_eq!(runs[2].text, "Connect what you've made.");
    }

    #[test]
    fn page_runs_url_above_title_above_body() {
        let runs = page_runs("u", "t", "b", 1000, 600);
        assert!(runs[0].y < runs[1].y);
        assert!(runs[1].y < runs[2].y);
    }

    #[test]
    fn page_runs_clip_within_surface() {
        let runs = page_runs("u", "t", "b", 800, 600);
        for r in &runs {
            assert!(r.x + r.width as f32 <= 800.0);
            assert!(r.y + r.height as f32 <= 600.0);
        }
    }

    #[test]
    fn empty_strings_still_produce_runs() {
        // Glyphon handles empty buffers; we shouldn't suppress
        // empty text either — the shell may legitimately want to
        // hide a section by passing empty.
        let runs = page_runs("", "", "", 1280, 800);
        assert_eq!(runs.len(), 3);
        for r in &runs {
            assert!(r.text.is_empty());
        }
    }

    #[test]
    fn small_window_doesnt_underflow_body_height() {
        // If the user resizes the window very small, body height
        // could underflow. Saturating math protects.
        let runs = page_runs("u", "t", "b", 200, 80);
        // Body run exists with non-negative height (saturating_sub).
        assert!(runs[2].height as i64 >= 0);
    }
}
