use tenui_anim::MicroStepper;
use tenui_core::{CanvasSubviewMut, Color, Modifier};

/// Canvas representation for continuous curves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlotCanvasType {
    #[default]
    Braille,
    HalfBlock,
}

/// Data series for Line and Scatter plots.
#[derive(Debug, Clone)]
pub struct Series {
    pub name: String,
    pub color: Color,
    pub points: Vec<(f64, f64)>,
}

impl Series {
    pub fn new(name: impl Into<String>, color: Color, points: Vec<(f64, f64)>) -> Self {
        Self {
            name: name.into(),
            color,
            points,
        }
    }
}

/// Multi-series continuous curve plot using Braille (2x4) or Half-Block resolution.
#[derive(Debug, Clone)]
pub struct LinePlot {
    pub series: Vec<Series>,
    pub canvas_type: PlotCanvasType,
    pub x_bounds: Option<(f64, f64)>,
    pub y_bounds: Option<(f64, f64)>,
    pub show_axes: bool,
    pub show_legend: bool,
}

impl Default for LinePlot {
    fn default() -> Self {
        Self {
            series: Vec::new(),
            canvas_type: PlotCanvasType::Braille,
            x_bounds: None,
            y_bounds: None,
            show_axes: true,
            show_legend: true,
        }
    }
}

impl LinePlot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    pub fn with_canvas_type(mut self, canvas_type: PlotCanvasType) -> Self {
        self.canvas_type = canvas_type;
        self
    }

    pub fn with_x_bounds(mut self, min: f64, max: f64) -> Self {
        self.x_bounds = Some((min, max));
        self
    }

    pub fn with_y_bounds(mut self, min: f64, max: f64) -> Self {
        self.y_bounds = Some((min, max));
        self
    }

    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>) {
        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        if w < 6 || h < 4 {
            return;
        }

        // Determine bounds
        let (min_x, max_x) = self.x_bounds.unwrap_or_else(|| {
            let mut min_val = f64::MAX;
            let mut max_val = f64::MIN;
            for s in &self.series {
                for &(x, _) in &s.points {
                    min_val = min_val.min(x);
                    max_val = max_val.max(x);
                }
            }
            if min_val >= max_val {
                (0.0, 1.0)
            } else {
                (min_val, max_val)
            }
        });

        let (min_y, max_y) = self.y_bounds.unwrap_or_else(|| {
            let mut min_val = f64::MAX;
            let mut max_val = f64::MIN;
            for s in &self.series {
                for &(_, y) in &s.points {
                    min_val = min_val.min(y);
                    max_val = max_val.max(y);
                }
            }
            if min_val >= max_val {
                (0.0, 1.0)
            } else {
                (min_val, max_val)
            }
        });

        // Layout: Left margin for Y-axis (6 cols), bottom margin for X-axis (1 row), optional legend at top (1 row)
        let y_margin = if self.show_axes { 6 } else { 0 };
        let x_margin = if self.show_axes { 1 } else { 0 };
        let legend_h = if self.show_legend && !self.series.is_empty() {
            1
        } else {
            0
        };

        let plot_w = w.saturating_sub(y_margin);
        let plot_h = h.saturating_sub(x_margin + legend_h);
        if plot_w == 0 || plot_h == 0 {
            return;
        }

        // Clear canvas
        for y in 0..canvas.height() {
            for x in 0..canvas.width() {
                canvas.set_char(x, y, ' ', Color::Reset, Color::Reset, Modifier::empty());
            }
        }

        // Render legend at row 0 if enabled
        if legend_h > 0 {
            let mut col = y_margin;
            for s in &self.series {
                if col + s.name.len() + 3 >= w {
                    break;
                }
                canvas.set_char(col as u16, 0, '■', s.color, Color::Reset, Modifier::empty());
                col += 2;
                for ch in s.name.chars() {
                    canvas.set_char(col as u16, 0, ch, Color::Reset, Color::Reset, Modifier::empty());
                    col += 1;
                }
                col += 2;
            }
        }

        // Render Y-axis ticks and vertical axis
        let plot_top = legend_h;
        let plot_bottom = plot_top + plot_h - 1;

        if self.show_axes {
            let top_val = format!("{:>5.1}│", max_y);
            let bot_val = format!("{:>5.1}│", min_y);
            for (i, ch) in top_val.chars().enumerate() {
                canvas.set_char(
                    i as u16,
                    plot_top as u16,
                    ch,
                    Color::DarkGray,
                    Color::Reset,
                    Modifier::empty(),
                );
            }
            for py in (plot_top + 1)..plot_bottom {
                canvas.set_char(
                    (y_margin - 1) as u16,
                    py as u16,
                    '│',
                    Color::DarkGray,
                    Color::Reset,
                    Modifier::empty(),
                );
            }
            for (i, ch) in bot_val.chars().enumerate() {
                canvas.set_char(
                    i as u16,
                    plot_bottom as u16,
                    ch,
                    Color::DarkGray,
                    Color::Reset,
                    Modifier::empty(),
                );
            }

            // X-axis horizontal line and ticks
            let axis_y = (plot_bottom + 1) as u16;
            canvas.set_char(
                (y_margin - 1) as u16,
                axis_y,
                '└',
                Color::DarkGray,
                Color::Reset,
                Modifier::empty(),
            );
            for px in 0..plot_w {
                canvas.set_char(
                    (y_margin + px) as u16,
                    axis_y,
                    '─',
                    Color::DarkGray,
                    Color::Reset,
                    Modifier::empty(),
                );
            }
        }

        // Render curves
        match self.canvas_type {
            PlotCanvasType::Braille => {
                // Braille grid: width * 2 dots, height * 4 dots
                let dot_w = plot_w * 2;
                let dot_h = plot_h * 4;
                let mut braille_grid = vec![vec![0u8; plot_w]; plot_h];
                let mut cell_colors = vec![vec![Color::Reset; plot_w]; plot_h];

                // Braille dot bitmask mapping:
                // Dot 1: (0,0) -> 0x01
                // Dot 2: (0,1) -> 0x02
                // Dot 3: (0,2) -> 0x04
                // Dot 4: (1,0) -> 0x08
                // Dot 5: (1,1) -> 0x10
                // Dot 6: (1,2) -> 0x20
                // Dot 7: (0,3) -> 0x40
                // Dot 8: (1,3) -> 0x80
                let dot_mask = |dx: usize, dy: usize| -> u8 {
                    match (dx, dy) {
                        (0, 0) => 0x01,
                        (0, 1) => 0x02,
                        (0, 2) => 0x04,
                        (1, 0) => 0x08,
                        (1, 1) => 0x10,
                        (1, 2) => 0x20,
                        (0, 3) => 0x40,
                        (1, 3) => 0x80,
                        _ => 0,
                    }
                };

                for s in &self.series {
                    if s.points.is_empty() {
                        continue;
                    }
                    for i in 0..s.points.len() {
                        let (x, y) = s.points[i];
                        let norm_x = ((x - min_x) / (max_x - min_x)).clamp(0.0, 1.0);
                        let norm_y = ((y - min_y) / (max_y - min_y)).clamp(0.0, 1.0);

                        let px = ((norm_x * (dot_w - 1) as f64).round() as usize).min(dot_w - 1);
                        let py = (((1.0 - norm_y) * (dot_h - 1) as f64).round() as usize).min(dot_h - 1);

                        let cell_x = px / 2;
                        let cell_y = py / 4;
                        let dx = px % 2;
                        let dy = py % 4;

                        if cell_x < plot_w && cell_y < plot_h {
                            braille_grid[cell_y][cell_x] |= dot_mask(dx, dy);
                            cell_colors[cell_y][cell_x] = s.color;
                        }
                    }
                }

                // Blit braille cells
                for cy in 0..plot_h {
                    for cx in 0..plot_w {
                        let mask = braille_grid[cy][cx];
                        if mask != 0 {
                            let ch = char::from_u32(0x2800 + mask as u32).unwrap_or(' ');
                            let color = cell_colors[cy][cx];
                            canvas.set_char(
                                (y_margin + cx) as u16,
                                (plot_top + cy) as u16,
                                ch,
                                color,
                                Color::Reset,
                                Modifier::empty(),
                            );
                        }
                    }
                }
            }
            PlotCanvasType::HalfBlock => {
                // Half-block grid: 2 dots vertically per cell ('▀' or '▄' or '█')
                let hb_h = plot_h * 2;
                let mut top_filled = vec![vec![false; plot_w]; plot_h];
                let mut bot_filled = vec![vec![false; plot_w]; plot_h];
                let mut cell_colors = vec![vec![Color::Reset; plot_w]; plot_h];

                for s in &self.series {
                    for &(x, y) in &s.points {
                        let norm_x = ((x - min_x) / (max_x - min_x)).clamp(0.0, 1.0);
                        let norm_y = ((y - min_y) / (max_y - min_y)).clamp(0.0, 1.0);

                        let cx = ((norm_x * (plot_w - 1) as f64).round() as usize).min(plot_w - 1);
                        let hy = (((1.0 - norm_y) * (hb_h - 1) as f64).round() as usize).min(hb_h - 1);

                        let cy = hy / 2;
                        let is_bot = (hy % 2) == 1;

                        if cy < plot_h && cx < plot_w {
                            if is_bot {
                                bot_filled[cy][cx] = true;
                            } else {
                                top_filled[cy][cx] = true;
                            }
                            cell_colors[cy][cx] = s.color;
                        }
                    }
                }

                for cy in 0..plot_h {
                    for cx in 0..plot_w {
                        let t = top_filled[cy][cx];
                        let b = bot_filled[cy][cx];
                        let ch = match (t, b) {
                            (true, true) => '█',
                            (true, false) => '▀',
                            (false, true) => '▄',
                            (false, false) => ' ',
                        };
                        if ch != ' ' {
                            canvas.set_char(
                                (y_margin + cx) as u16,
                                (plot_top + cy) as u16,
                                ch,
                                cell_colors[cy][cx],
                                Color::Reset,
                                Modifier::empty(),
                            );
                        }
                    }
                }
            }
        }
    }
}

/// ScatterPlot for discrete 2D point cloud visualization.
#[derive(Debug, Clone)]
pub struct ScatterPlot {
    pub series: Vec<Series>,
    pub marker: char,
}

impl ScatterPlot {
    pub fn new() -> Self {
        Self {
            series: Vec::new(),
            marker: '•',
        }
    }

    pub fn add_series(mut self, series: Series) -> Self {
        self.series.push(series);
        self
    }

    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>) {
        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        if w == 0 || h == 0 || self.series.is_empty() {
            return;
        }

        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;

        for s in &self.series {
            for &(x, y) in &s.points {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }

        if min_x >= max_x {
            max_x = min_x + 1.0;
        }
        if min_y >= max_y {
            max_y = min_y + 1.0;
        }

        for s in &self.series {
            for &(x, y) in &s.points {
                let norm_x = ((x - min_x) / (max_x - min_x)).clamp(0.0, 1.0);
                let norm_y = ((y - min_y) / (max_y - min_y)).clamp(0.0, 1.0);

                let cx = ((norm_x * (w - 1) as f64).round() as usize).min(w - 1);
                let cy = (((1.0 - norm_y) * (h - 1) as f64).round() as usize).min(h - 1);

                canvas.set_char(
                    cx as u16,
                    cy as u16,
                    self.marker,
                    s.color,
                    Color::Reset,
                    Modifier::empty(),
                );
            }
        }
    }
}

impl Default for ScatterPlot {
    fn default() -> Self {
        Self::new()
    }
}

/// Bar chart item.
#[derive(Debug, Clone)]
pub struct BarItem {
    pub label: String,
    pub value: f64,
    pub color: Color,
}

/// Bar orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// BarChart utilizing Unicode 1/8th sub-character micro-stepping for smooth fractional bars.
#[derive(Debug, Clone)]
pub struct BarChart {
    pub items: Vec<BarItem>,
    pub orientation: BarOrientation,
}

impl BarChart {
    pub fn new(items: Vec<BarItem>) -> Self {
        Self {
            items,
            orientation: BarOrientation::Horizontal,
        }
    }

    pub fn with_orientation(mut self, orientation: BarOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>) {
        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        if w == 0 || h == 0 || self.items.is_empty() {
            return;
        }

        let max_val = self.items.iter().map(|it| it.value).fold(0.0f64, f64::max).max(1.0);

        match self.orientation {
            BarOrientation::Horizontal => {
                let label_w = self.items.iter().map(|it| it.label.len()).max().unwrap_or(4).min(15);
                let bar_max_w = w.saturating_sub(label_w + 2);

                for (i, item) in self.items.iter().enumerate() {
                    if i >= h {
                        break;
                    }
                    let y = i as u16;

                    // Draw label
                    let label_str = format!("{:<label_w$} │", &item.label[..item.label.len().min(label_w)]);
                    for (x, ch) in label_str.chars().enumerate() {
                        if x < w {
                            canvas.set_char(x as u16, y, ch, Color::Reset, Color::Reset, Modifier::empty());
                        }
                    }

                    // Render horizontal micro-stepped bar
                    let norm = (item.value / max_val).clamp(0.0, 1.0);
                    let bar_len = norm as f32 * bar_max_w as f32;
                    let micro = MicroStepper::resolve_horizontal(bar_len);

                    let bar_start = label_w + 2;
                    for x_offset in 0..micro.whole_cells {
                        let target_x = bar_start + x_offset as usize;
                        if target_x >= w {
                            break;
                        }
                        canvas.set_char(target_x as u16, y, '█', item.color, Color::Reset, Modifier::empty());
                    }
                    if let Some(glyph) = micro.fractional_glyph {
                        let target_x = bar_start + micro.whole_cells as usize;
                        if target_x < w {
                            canvas.set_char(target_x as u16, y, glyph, item.color, Color::Reset, Modifier::empty());
                        }
                    }
                }
            }
            BarOrientation::Vertical => {
                let bar_count = self.items.len();
                let col_w = (w / bar_count).max(1);
                let bar_max_h = h.saturating_sub(1);

                for (i, item) in self.items.iter().enumerate() {
                    let col_start = i * col_w;
                    if col_start >= w {
                        break;
                    }

                    let norm = (item.value / max_val).clamp(0.0, 1.0);
                    let bar_len = norm as f32 * bar_max_h as f32;
                    let micro = MicroStepper::resolve_vertical(bar_len);

                    // Draw whole blocks from bottom up
                    for y_offset in 0..micro.whole_cells {
                        let y = (bar_max_h - 1).saturating_sub(y_offset as usize) as u16;
                        for dx in 0..col_w.min(2) {
                            let x = (col_start + dx) as u16;
                            if (x as usize) < w {
                                canvas.set_char(x, y, '█', item.color, Color::Reset, Modifier::empty());
                            }
                        }
                    }

                    // Draw fractional top glyph
                    if let Some(glyph) = micro.fractional_glyph {
                        let y = (bar_max_h - 1).saturating_sub(micro.whole_cells as usize) as u16;
                        for dx in 0..col_w.min(2) {
                            let x = (col_start + dx) as u16;
                            if (x as usize) < w {
                                canvas.set_char(x, y, glyph, item.color, Color::Reset, Modifier::empty());
                            }
                        }
                    }

                    // Draw bottom label
                    let label_char = item.label.chars().next().unwrap_or(' ');
                    canvas.set_char(
                        col_start as u16,
                        (h - 1) as u16,
                        label_char,
                        Color::DarkGray,
                        Color::Reset,
                        Modifier::empty(),
                    );
                }
            }
        }
    }
}

/// Stacked bar segment.
#[derive(Debug, Clone)]
pub struct StackedSegment {
    pub value: f64,
    pub color: Color,
}

/// StackedBarChart showing composite distributions.
#[derive(Debug, Clone)]
pub struct StackedBarChart {
    pub categories: Vec<(String, Vec<StackedSegment>)>,
}

impl StackedBarChart {
    pub fn new(categories: Vec<(String, Vec<StackedSegment>)>) -> Self {
        Self { categories }
    }

    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>) {
        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        if w == 0 || h == 0 || self.categories.is_empty() {
            return;
        }

        let max_total = self
            .categories
            .iter()
            .map(|(_, segs)| segs.iter().map(|s| s.value).sum::<f64>())
            .fold(0.0, f64::max)
            .max(1.0);
        let label_w = self.categories.iter().map(|(l, _)| l.len()).max().unwrap_or(4).min(12);
        let bar_max_w = w.saturating_sub(label_w + 2);

        for (i, (label, segs)) in self.categories.iter().enumerate() {
            if i >= h {
                break;
            }
            let y = i as u16;

            let label_fmt = format!("{:<label_w$} │", &label[..label.len().min(label_w)]);
            for (cx, ch) in label_fmt.chars().enumerate() {
                if cx < w {
                    canvas.set_char(cx as u16, y, ch, Color::Reset, Color::Reset, Modifier::empty());
                }
            }

            let mut cur_x = label_w + 2;
            for seg in segs {
                let seg_w = ((seg.value / max_total) * bar_max_w as f64).round() as usize;
                for _ in 0..seg_w {
                    if cur_x < w {
                        canvas.set_char(cur_x as u16, y, '█', seg.color, Color::Reset, Modifier::empty());
                        cur_x += 1;
                    }
                }
            }
        }
    }
}

/// Single financial/telemetry OHLC candle.
#[derive(Debug, Clone, Copy)]
pub struct Candle {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// CandlestickChart for financial and telemetry metrics with volume bars.
#[derive(Debug, Clone)]
pub struct CandlestickChart {
    pub candles: Vec<Candle>,
    pub up_color: Color,
    pub down_color: Color,
    pub show_volume: bool,
}

impl CandlestickChart {
    pub fn new(candles: Vec<Candle>) -> Self {
        Self {
            candles,
            up_color: Color::Rgb(166, 227, 161),   // Green
            down_color: Color::Rgb(243, 139, 168), // Red
            show_volume: true,
        }
    }

    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>) {
        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        if w == 0 || h < 4 || self.candles.is_empty() {
            return;
        }

        let volume_h = if self.show_volume { 2 } else { 0 };
        let price_h = h.saturating_sub(volume_h);

        let min_price = self.candles.iter().map(|c| c.low).fold(f64::MAX, f64::min);
        let max_price = self.candles.iter().map(|c| c.high).fold(f64::MIN, f64::max);
        let max_vol = self.candles.iter().map(|c| c.volume).fold(0.0, f64::max).max(1.0);
        let range = (max_price - min_price).max(1e-6);

        let candle_count = self.candles.len();
        let col_w = (w / candle_count).max(1);

        for (i, c) in self.candles.iter().enumerate() {
            let cx = (i * col_w) as u16;
            if cx as usize >= w {
                break;
            }

            let is_up = c.close >= c.open;
            let color = if is_up { self.up_color } else { self.down_color };

            let to_y = |val: f64| -> u16 {
                let norm = ((val - min_price) / range).clamp(0.0, 1.0);
                (((1.0 - norm) * (price_h - 1) as f64).round() as u16).min((price_h - 1) as u16)
            };

            let high_y = to_y(c.high);
            let low_y = to_y(c.low);
            let open_y = to_y(c.open);
            let close_y = to_y(c.close);

            let body_top = open_y.min(close_y);
            let body_bot = open_y.max(close_y);

            // Draw upper and lower wicks
            for y in high_y..=low_y {
                canvas.set_char(cx, y, '│', color, Color::Reset, Modifier::empty());
            }

            // Draw body
            for y in body_top..=body_bot {
                canvas.set_char(cx, y, '█', color, Color::Reset, Modifier::empty());
            }

            // Draw volume bar
            if self.show_volume {
                let vol_norm = (c.volume / max_vol).clamp(0.0, 1.0);
                let v_rows = (vol_norm * volume_h as f64).round() as u16;
                for vy in 0..v_rows {
                    let y = (h - 1 - vy as usize) as u16;
                    canvas.set_char(cx, y, '▄', Color::DarkGray, Color::Reset, Modifier::empty());
                }
            }
        }
    }
}

/// Heatmap 2D matrix visualization with dynamic RGB perceptual gradients.
#[derive(Debug, Clone)]
pub struct Heatmap {
    pub data: Vec<Vec<f64>>,
    pub row_labels: Vec<String>,
    pub col_labels: Vec<String>,
    pub min_val: f64,
    pub max_val: f64,
}

impl Heatmap {
    pub fn new(data: Vec<Vec<f64>>) -> Self {
        let mut min_val = f64::MAX;
        let mut max_val = f64::MIN;
        for row in &data {
            for &val in row {
                min_val = min_val.min(val);
                max_val = max_val.max(val);
            }
        }
        if min_val >= max_val {
            max_val = min_val + 1.0;
        }

        Self {
            data,
            row_labels: Vec::new(),
            col_labels: Vec::new(),
            min_val,
            max_val,
        }
    }

    /// Maps normalized scalar t in [0, 1] to a thermal RGB gradient (Deep Blue -> Cyan -> Yellow -> Red).
    fn thermal_gradient(t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let r = (255.0 * (1.5 * t - 0.5).clamp(0.0, 1.0)) as u8;
        let g = (255.0 * (1.0 - (2.0 * t - 1.0).abs()).clamp(0.0, 1.0)) as u8;
        let b = (255.0 * (1.5 - 2.0 * t).clamp(0.0, 1.0)) as u8;
        Color::Rgb(r, g, b)
    }

    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>) {
        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        if w == 0 || h == 0 || self.data.is_empty() {
            return;
        }

        let num_rows = self.data.len();
        let num_cols = self.data[0].len();
        if num_cols == 0 {
            return;
        }

        let cell_w = (w / num_cols).max(1);
        let cell_h = (h / num_rows).max(1);

        for r in 0..num_rows {
            for c in 0..num_cols {
                let val = self.data[r][c];
                let norm = ((val - self.min_val) / (self.max_val - self.min_val)).clamp(0.0, 1.0) as f32;
                let bg = Self::thermal_gradient(norm);

                let start_x = c * cell_w;
                let start_y = r * cell_h;

                for dy in 0..cell_h {
                    for dx in 0..cell_w {
                        let px = (start_x + dx) as u16;
                        let py = (start_y + dy) as u16;
                        if (px as usize) < w && (py as usize) < h {
                            canvas.set_char(px, py, ' ', Color::Reset, bg, Modifier::empty());
                        }
                    }
                }
            }
        }
    }
}
