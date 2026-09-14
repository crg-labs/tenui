use std::time::{Duration, Instant};

use tenui_core::{buffer::CanvasSubviewMut, cell::Cell, color::Color};

/// Supported uncompressed pixel color formats for half-block conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb8,
    Rgba8,
}

/// A raw decoded video frame buffer.
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub width: u16,
    pub height: u16,
    pub format: PixelFormat,
    pub data: Vec<u8>,
    pub timestamp: Duration,
}

impl VideoFrame {
    pub fn new(width: u16, height: u16, format: PixelFormat, data: Vec<u8>, timestamp: Duration) -> Self {
        Self {
            width,
            height,
            format,
            data,
            timestamp,
        }
    }

    /// Convenience helper to create an RGB test frame filled with a solid color.
    pub fn solid_rgb(width: u16, height: u16, r: u8, g: u8, b: u8) -> Self {
        let size = (width as usize) * (height as usize) * 3;
        let mut data = Vec::with_capacity(size);
        for _ in 0..(width as usize * height as usize) {
            data.push(r);
            data.push(g);
            data.push(b);
        }
        Self {
            width,
            height,
            format: PixelFormat::Rgb8,
            data,
            timestamp: Duration::ZERO,
        }
    }
}

#[inline]
#[allow(clippy::too_many_arguments)]
fn sample_area_rgb(
    rgb: &[u8],
    stride: usize,
    src_w: usize,
    src_h: usize,
    x0: usize,
    x1: usize,
    y0: usize,
    y1: usize,
) -> (u8, u8, u8) {
    let x_end = x1.min(src_w.saturating_sub(1));
    let y_end = y1.min(src_h.saturating_sub(1));
    let mut r = 0u32;
    let mut g = 0u32;
    let mut b = 0u32;
    let mut count = 0u32;

    for y in y0..=y_end {
        let row_off = y * stride;
        for x in x0..=x_end {
            let p = row_off + x * 3;
            if p + 2 < rgb.len() {
                r += rgb[p] as u32;
                g += rgb[p + 1] as u32;
                b += rgb[p + 2] as u32;
                count += 1;
            }
        }
    }

    if count > 0 {
        let c = count.max(1);
        ((r / c) as u8, (g / c) as u8, (b / c) as u8)
    } else {
        (0, 0, 0)
    }
}

/// Converter translating 24-bit/32-bit pixel buffers into 2-pixel-per-cell Half-Block ('▀') TUI grids.
pub struct HalfBlockVideoConverter;

impl HalfBlockVideoConverter {
    /// Renders an RGB byte slice directly to a canvas subview at the given coordinates.
    ///
    /// The top subpixel determines `fg`, the bottom subpixel determines `bg`.
    /// Each character cell corresponds to a 1x2 pixel block.
    pub fn render_rgb(
        surface: &mut CanvasSubviewMut,
        origin_x: u16,
        origin_y: u16,
        img_w: u16,
        img_h: u16,
        rgb_data: &[u8],
    ) {
        let cell_rows = img_h.div_ceil(2);
        let cell_cols = img_w;
        let stride = (img_w as usize) * 3;

        for cy in 0..cell_rows {
            let py_top = (cy as usize) * 2;
            let py_bot = py_top + 1;

            let top_row_offset = py_top * stride;
            let bot_row_offset = if py_bot < img_h as usize {
                Some(py_bot * stride)
            } else {
                None
            };

            for cx in 0..cell_cols {
                let px = (cx as usize) * 3;
                let (r_top, g_top, b_top) = if top_row_offset + px + 2 < rgb_data.len() {
                    (
                        rgb_data[top_row_offset + px],
                        rgb_data[top_row_offset + px + 1],
                        rgb_data[top_row_offset + px + 2],
                    )
                } else {
                    (0, 0, 0)
                };

                let (r_bot, g_bot, b_bot) = match bot_row_offset {
                    Some(bot_offset) if bot_offset + px + 2 < rgb_data.len() => (
                        rgb_data[bot_offset + px],
                        rgb_data[bot_offset + px + 1],
                        rgb_data[bot_offset + px + 2],
                    ),
                    _ => (0, 0, 0),
                };

                let mut cell = Cell::default();
                cell.set_char('▀');
                cell.set_fg(Color::Rgb(r_top, g_top, b_top));
                cell.set_bg(Color::Rgb(r_bot, g_bot, b_bot));

                let target_x = origin_x + cx;
                let target_y = origin_y + cy;
                surface.set_cell_clipped(target_x, target_y, cell);
            }
        }
    }

    /// Renders an RGB byte slice scaled to fit destination bounds (dest_w x dest_h cells)
    /// with anti-aliasing area averaging.
    #[allow(clippy::too_many_arguments)]
    pub fn render_rgb_scaled(
        surface: &mut CanvasSubviewMut,
        origin_x: u16,
        origin_y: u16,
        dest_w: u16,
        dest_h: u16,
        src_w: u16,
        src_h: u16,
        rgb_data: &[u8],
    ) {
        if dest_w == 0 || dest_h == 0 || src_w == 0 || src_h == 0 || rgb_data.is_empty() {
            return;
        }

        let stride = (src_w as usize) * 3;
        let sub_height = (dest_h as usize) * 2;

        for cy in 0..dest_h {
            let sy0_top = (cy as usize * 2 * src_h as usize) / sub_height;
            let sy1_top = ((cy as usize * 2 + 1) * src_h as usize) / sub_height;

            let sy0_bot = ((cy as usize * 2 + 1) * src_h as usize) / sub_height;
            let sy1_bot = ((cy as usize * 2 + 2) * src_h as usize) / sub_height;

            for cx in 0..dest_w {
                let sx0 = (cx as usize * src_w as usize) / dest_w as usize;
                let sx1 = ((cx as usize + 1) * src_w as usize) / dest_w as usize;

                let (r_top, g_top, b_top) = sample_area_rgb(
                    rgb_data,
                    stride,
                    src_w as usize,
                    src_h as usize,
                    sx0,
                    sx1,
                    sy0_top,
                    sy1_top,
                );

                let (r_bot, g_bot, b_bot) = sample_area_rgb(
                    rgb_data,
                    stride,
                    src_w as usize,
                    src_h as usize,
                    sx0,
                    sx1,
                    sy0_bot,
                    sy1_bot,
                );

                let mut cell = Cell::default();
                cell.set_char('▀');
                cell.set_fg(Color::Rgb(r_top, g_top, b_top));
                cell.set_bg(Color::Rgb(r_bot, g_bot, b_bot));

                let target_x = origin_x + cx;
                let target_y = origin_y + cy;
                surface.set_cell_clipped(target_x, target_y, cell);
            }
        }
    }

    /// Renders an RGB byte slice scaled to fit destination bounds while strictly preserving 16:9 aspect ratio,
    /// centering the video and padding with black letterbox / pillarbox bars.
    #[allow(clippy::too_many_arguments)]
    pub fn render_rgb_fitted_16_9(
        surface: &mut CanvasSubviewMut,
        origin_x: u16,
        origin_y: u16,
        dest_w: u16,
        dest_h: u16,
        src_w: u16,
        src_h: u16,
        rgb_data: &[u8],
    ) {
        if dest_w == 0 || dest_h == 0 || src_w == 0 || src_h == 0 || rgb_data.is_empty() {
            return;
        }

        let avail_sub_w = dest_w as f32;
        let avail_sub_h = (dest_h as f32) * 2.0;
        let target_aspect = (src_w as f32) / (src_h as f32);

        let (fit_sub_w, fit_sub_h) = if (avail_sub_w / avail_sub_h) > target_aspect {
            let fit_h = avail_sub_h;
            let fit_w = (fit_h * target_aspect).round().min(avail_sub_w);
            (fit_w as u16, fit_h as u16)
        } else {
            let fit_w = avail_sub_w;
            let fit_h = (fit_w / target_aspect).round().min(avail_sub_h);
            (fit_w as u16, fit_h as u16)
        };

        let fit_cell_w = fit_sub_w.max(1);
        let fit_cell_h = (fit_sub_h / 2).max(1);

        let pad_x = (dest_w.saturating_sub(fit_cell_w)) / 2;
        let pad_y = (dest_h.saturating_sub(fit_cell_h)) / 2;

        // Clear background / letterbox bars to black
        for y in 0..dest_h {
            for x in 0..dest_w {
                if x < pad_x || x >= pad_x + fit_cell_w || y < pad_y || y >= pad_y + fit_cell_h {
                    let mut black_cell = Cell::default();
                    black_cell.set_char(' ');
                    black_cell.set_fg(Color::Black);
                    black_cell.set_bg(Color::Black);
                    surface.set_cell_clipped(origin_x + x, origin_y + y, black_cell);
                }
            }
        }

        Self::render_rgb_scaled(
            surface,
            origin_x + pad_x,
            origin_y + pad_y,
            fit_cell_w,
            fit_cell_h,
            src_w,
            src_h,
            rgb_data,
        );
    }

    /// Renders an RGBA byte slice directly to a canvas subview at the given coordinates.
    pub fn render_rgba(
        surface: &mut CanvasSubviewMut,
        origin_x: u16,
        origin_y: u16,
        img_w: u16,
        img_h: u16,
        rgba_data: &[u8],
    ) {
        let cell_rows = img_h.div_ceil(2);
        let cell_cols = img_w;
        let stride = (img_w as usize) * 4;

        for cy in 0..cell_rows {
            let py_top = (cy as usize) * 2;
            let py_bot = py_top + 1;

            let top_row_offset = py_top * stride;
            let bot_row_offset = if py_bot < img_h as usize {
                Some(py_bot * stride)
            } else {
                None
            };

            for cx in 0..cell_cols {
                let px = (cx as usize) * 4;
                let (r_top, g_top, b_top) = if top_row_offset + px + 3 < rgba_data.len() {
                    (
                        rgba_data[top_row_offset + px],
                        rgba_data[top_row_offset + px + 1],
                        rgba_data[top_row_offset + px + 2],
                    )
                } else {
                    (0, 0, 0)
                };

                let (r_bot, g_bot, b_bot) = match bot_row_offset {
                    Some(bot_offset) if bot_offset + px + 3 < rgba_data.len() => (
                        rgba_data[bot_offset + px],
                        rgba_data[bot_offset + px + 1],
                        rgba_data[bot_offset + px + 2],
                    ),
                    _ => (0, 0, 0),
                };

                let mut cell = Cell::default();
                cell.set_char('▀');
                cell.set_fg(Color::Rgb(r_top, g_top, b_top));
                cell.set_bg(Color::Rgb(r_bot, g_bot, b_bot));

                let target_x = origin_x + cx;
                let target_y = origin_y + cy;
                surface.set_cell_clipped(target_x, target_y, cell);
            }
        }
    }

    /// Renders a full `VideoFrame` onto the canvas subview.
    pub fn render_frame(surface: &mut CanvasSubviewMut, origin_x: u16, origin_y: u16, frame: &VideoFrame) {
        match frame.format {
            PixelFormat::Rgb8 => {
                Self::render_rgb(surface, origin_x, origin_y, frame.width, frame.height, &frame.data);
            }
            PixelFormat::Rgba8 => {
                Self::render_rgba(surface, origin_x, origin_y, frame.width, frame.height, &frame.data);
            }
        }
    }

    /// Converts a video frame into a flat linear vector of `Cell`s of size (width * ((height + 1) / 2)).
    pub fn convert_frame_to_cells(frame: &VideoFrame) -> Vec<Cell> {
        let cell_rows = frame.height.div_ceil(2);
        let cell_cols = frame.width;
        let mut cells = Vec::with_capacity((cell_rows as usize) * (cell_cols as usize));

        let bytes_per_pixel = match frame.format {
            PixelFormat::Rgb8 => 3,
            PixelFormat::Rgba8 => 4,
        };
        let stride = (frame.width as usize) * bytes_per_pixel;

        for cy in 0..cell_rows {
            let py_top = (cy as usize) * 2;
            let py_bot = py_top + 1;

            let top_row_offset = py_top * stride;
            let bot_row_offset = if py_bot < frame.height as usize {
                Some(py_bot * stride)
            } else {
                None
            };

            for cx in 0..cell_cols {
                let px = (cx as usize) * bytes_per_pixel;
                let (r_top, g_top, b_top) = if top_row_offset + px + 2 < frame.data.len() {
                    (
                        frame.data[top_row_offset + px],
                        frame.data[top_row_offset + px + 1],
                        frame.data[top_row_offset + px + 2],
                    )
                } else {
                    (0, 0, 0)
                };

                let (r_bot, g_bot, b_bot) = match bot_row_offset {
                    Some(bot_offset) if bot_offset + px + 2 < frame.data.len() => (
                        frame.data[bot_offset + px],
                        frame.data[bot_offset + px + 1],
                        frame.data[bot_offset + px + 2],
                    ),
                    _ => (0, 0, 0),
                };

                let mut cell = Cell::default();
                cell.set_char('▀');
                cell.set_fg(Color::Rgb(r_top, g_top, b_top));
                cell.set_bg(Color::Rgb(r_bot, g_bot, b_bot));
                cells.push(cell);
            }
        }

        cells
    }
}

/// Dynamic backpressure and frame regulator for streaming video into terminal environments.
#[derive(Debug, Clone)]
pub struct MediaStreamController {
    pub target_fps: u32,
    pub max_latency_threshold: Duration,
    pub frames_received: u64,
    pub frames_rendered: u64,
    pub frames_dropped: u64,
    pub last_frame_time: Option<Instant>,
}

impl MediaStreamController {
    /// Creates a new controller with the specified target FPS and maximum tolerable acknowledgement latency.
    pub fn new(target_fps: u32, max_latency_threshold: Duration) -> Self {
        Self {
            target_fps,
            max_latency_threshold,
            frames_received: 0,
            frames_rendered: 0,
            frames_dropped: 0,
            last_frame_time: None,
        }
    }

    /// Evaluates socket acknowledgement / render latency.
    ///
    /// If `ack_latency > max_latency_threshold`, the frame is dropped to prevent PTY buffer saturation.
    /// Returns `true` if the frame should be dropped, or `false` if it should be rendered.
    pub fn should_drop_frame(&mut self, ack_latency: Duration) -> bool {
        self.frames_received += 1;
        if ack_latency > self.max_latency_threshold {
            self.frames_dropped += 1;
            true
        } else {
            self.frames_rendered += 1;
            self.last_frame_time = Some(Instant::now());
            false
        }
    }

    /// Ratio of dropped frames to total received frames [0.0, 1.0].
    pub fn drop_ratio(&self) -> f32 {
        if self.frames_received == 0 {
            0.0
        } else {
            self.frames_dropped as f32 / self.frames_received as f32
        }
    }

    /// Calculates the effective rendering frame rate over the specified elapsed time.
    pub fn effective_fps(&self, elapsed: Duration) -> f32 {
        let secs = elapsed.as_secs_f32();
        if secs <= 0.0 {
            0.0
        } else {
            self.frames_rendered as f32 / secs
        }
    }

    /// Resets frame counters.
    pub fn reset_metrics(&mut self) {
        self.frames_received = 0;
        self.frames_rendered = 0;
        self.frames_dropped = 0;
    }
}

#[cfg(test)]
mod tests {
    use tenui_core::{Buffer, Rect};

    use super::*;

    #[test]
    fn test_half_block_rendering() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 4));
        let mut subview = buffer.subview_mut(Rect::new(0, 0, 4, 4));

        // 4x2 image: row 0 is red (255, 0, 0), row 1 is blue (0, 0, 255)
        let mut rgb = Vec::new();
        for _ in 0..4 {
            rgb.extend_from_slice(&[255, 0, 0]);
        }
        for _ in 0..4 {
            rgb.extend_from_slice(&[0, 0, 255]);
        }

        HalfBlockVideoConverter::render_rgb(&mut subview, 0, 0, 4, 2, &rgb);

        // Should produce 1 row of cells (cy = 0) where char is '▀', fg is red, bg is blue
        let cell = subview.get(0, 0).expect("cell exists");
        assert_eq!(cell.symbol.as_str(), "▀");
        assert_eq!(cell.fg, Color::Rgb(255, 0, 0));
        assert_eq!(cell.bg, Color::Rgb(0, 0, 255));
    }

    #[test]
    fn test_backpressure_frame_dropping() {
        let mut controller = MediaStreamController::new(60, Duration::from_millis(30));

        // Under threshold -> rendered
        assert!(!controller.should_drop_frame(Duration::from_millis(15)));
        assert_eq!(controller.frames_rendered, 1);
        assert_eq!(controller.frames_dropped, 0);

        // Exceeds threshold -> dropped
        assert!(controller.should_drop_frame(Duration::from_millis(45)));
        assert_eq!(controller.frames_rendered, 1);
        assert_eq!(controller.frames_dropped, 1);

        assert_eq!(controller.drop_ratio(), 0.5);
    }

    #[test]
    fn test_half_block_scaled_rendering() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 8, 4));
        let mut subview = buffer.subview_mut(Rect::new(0, 0, 8, 4));

        // 2x2 image scaled up to 8x4 cells
        let rgb = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 0];

        HalfBlockVideoConverter::render_rgb_scaled(&mut subview, 0, 0, 8, 4, 2, 2, &rgb);
        let cell = subview.get(0, 0).expect("cell exists");
        assert_eq!(cell.symbol.as_str(), "▀");
    }
}
