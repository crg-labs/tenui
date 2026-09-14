use std::thread;

use tenui_core::{CanvasSubviewMut, Color, Modifier};

/// Diff layout visualization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffMode {
    #[default]
    Unified,
    Split,
}

/// Type of change for a diff line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Equal,
    Insert,
    Delete,
}

/// A line change with intra-line character modification spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub change: ChangeType,
    pub content: String,
    pub old_line_num: Option<usize>,
    pub new_line_num: Option<usize>,
    /// Byte ranges within `content` that were specifically added/deleted intra-line.
    pub intra_highlights: Vec<(usize, usize)>,
}

/// Hunk header describing a block of diff changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub lines: Vec<DiffLine>,
}

/// Myers diff algorithm implementation producing line diffs and intra-line highlights.
/// Highlights represented by (start_col, end_col) ranges for old and new lines.
pub type IntraLineHighlights = (Vec<(usize, usize)>, Vec<(usize, usize)>);

pub struct MyersDiff;

impl MyersDiff {
    pub fn diff(old_text: &str, new_text: &str) -> Vec<DiffHunk> {
        let old_lines: Vec<&str> = old_text.lines().collect();
        let new_lines: Vec<&str> = new_text.lines().collect();

        let n = old_lines.len();
        let m = new_lines.len();
        if n == 0 && m == 0 {
            return Vec::new();
        }

        let max = n + m;
        let offset = max as isize;
        let mut v = vec![0isize; 2 * max + 2];
        let mut trace = Vec::new();

        'outer: for d in 0..=max {
            let mut v_snap = v.clone();
            for k in (-(d as isize)..=(d as isize)).step_by(2) {
                let idx = (k + offset) as usize;
                let mut x = if k == -(d as isize)
                    || (k != d as isize && v[(k - 1 + offset) as usize] < v[(k + 1 + offset) as usize])
                {
                    v[(k + 1 + offset) as usize]
                } else {
                    v[(k - 1 + offset) as usize] + 1
                };
                let mut y = x - k;

                while x < n as isize && y < m as isize && old_lines[x as usize] == new_lines[y as usize] {
                    x += 1;
                    y += 1;
                }

                v_snap[idx] = x;

                if x >= n as isize && y >= m as isize {
                    trace.push(v_snap);
                    break 'outer;
                }
            }
            v = v_snap.clone();
            trace.push(v_snap);
        }

        // Backtrack edit script
        let mut x = n as isize;
        let mut y = m as isize;
        let mut script = Vec::new();

        for d in (1..trace.len()).rev() {
            let v_prev = &trace[d - 1];
            let k = x - y;
            let idx_k_prev_down = (k + 1 + offset) as usize;
            let idx_k_prev_up = (k - 1 + offset) as usize;

            let prev_k = if k == -(d as isize) || (k != d as isize && v_prev[idx_k_prev_up] < v_prev[idx_k_prev_down]) {
                k + 1
            } else {
                k - 1
            };

            let prev_x = v_prev[(prev_k + offset) as usize];
            let prev_y = prev_x - prev_k;

            while x > prev_x && y > prev_y {
                script.push((
                    ChangeType::Equal,
                    old_lines[(x - 1) as usize],
                    (x - 1) as usize,
                    (y - 1) as usize,
                ));
                x -= 1;
                y -= 1;
            }

            if d > 0 {
                if prev_k < k {
                    if x > 0 {
                        script.push((
                            ChangeType::Delete,
                            old_lines[(x - 1) as usize],
                            (x - 1) as usize,
                            y as usize,
                        ));
                    }
                } else if prev_k > k && y > 0 {
                    script.push((
                        ChangeType::Insert,
                        new_lines[(y - 1) as usize],
                        x as usize,
                        (y - 1) as usize,
                    ));
                }
            }
            x = prev_x;
            y = prev_y;
        }

        while x > 0 && y > 0 {
            script.push((
                ChangeType::Equal,
                old_lines[(x - 1) as usize],
                (x - 1) as usize,
                (y - 1) as usize,
            ));
            x -= 1;
            y -= 1;
        }

        script.reverse();

        // Convert script into DiffHunks and calculate intra-line edits
        let mut lines = Vec::new();
        let mut old_idx = 1;
        let mut new_idx = 1;

        let mut i = 0;
        while i < script.len() {
            let (change, content, _, _) = script[i];
            match change {
                ChangeType::Equal => {
                    lines.push(DiffLine {
                        change: ChangeType::Equal,
                        content: content.to_string(),
                        old_line_num: Some(old_idx),
                        new_line_num: Some(new_idx),
                        intra_highlights: Vec::new(),
                    });
                    old_idx += 1;
                    new_idx += 1;
                    i += 1;
                }
                ChangeType::Delete => {
                    if i + 1 < script.len() && script[i + 1].0 == ChangeType::Insert {
                        let del_content = content;
                        let ins_content = script[i + 1].1;
                        let (del_spans, ins_spans) = Self::intra_line_diff(del_content, ins_content);

                        lines.push(DiffLine {
                            change: ChangeType::Delete,
                            content: del_content.to_string(),
                            old_line_num: Some(old_idx),
                            new_line_num: None,
                            intra_highlights: del_spans,
                        });
                        old_idx += 1;

                        lines.push(DiffLine {
                            change: ChangeType::Insert,
                            content: ins_content.to_string(),
                            old_line_num: None,
                            new_line_num: Some(new_idx),
                            intra_highlights: ins_spans,
                        });
                        new_idx += 1;
                        i += 2;
                    } else {
                        lines.push(DiffLine {
                            change: ChangeType::Delete,
                            content: content.to_string(),
                            old_line_num: Some(old_idx),
                            new_line_num: None,
                            intra_highlights: vec![(0, content.len())],
                        });
                        old_idx += 1;
                        i += 1;
                    }
                }
                ChangeType::Insert => {
                    lines.push(DiffLine {
                        change: ChangeType::Insert,
                        content: content.to_string(),
                        old_line_num: None,
                        new_line_num: Some(new_idx),
                        intra_highlights: vec![(0, content.len())],
                    });
                    new_idx += 1;
                    i += 1;
                }
            }
        }

        if lines.is_empty() {
            return Vec::new();
        }

        vec![DiffHunk {
            old_start: 1,
            old_count: n,
            new_start: 1,
            new_count: m,
            lines,
        }]
    }

    /// Performs sub-token intra-line character Myers diffing between old and new strings.
    fn intra_line_diff(old: &str, new: &str) -> IntraLineHighlights {
        let old_chars: Vec<char> = old.chars().collect();
        let new_chars: Vec<char> = new.chars().collect();

        let mut prefix = 0;
        while prefix < old_chars.len() && prefix < new_chars.len() && old_chars[prefix] == new_chars[prefix] {
            prefix += 1;
        }

        let mut suffix = 0;
        while suffix < (old_chars.len() - prefix)
            && suffix < (new_chars.len() - prefix)
            && old_chars[old_chars.len() - 1 - suffix] == new_chars[new_chars.len() - 1 - suffix]
        {
            suffix += 1;
        }

        let old_del_start = prefix;
        let old_del_end = old_chars.len() - suffix;
        let new_ins_start = prefix;
        let new_ins_end = new_chars.len() - suffix;

        let del_spans = if old_del_start < old_del_end {
            let b_start: usize = old_chars[..old_del_start].iter().map(|c| c.len_utf8()).sum();
            let b_len: usize = old_chars[old_del_start..old_del_end].iter().map(|c| c.len_utf8()).sum();
            vec![(b_start, b_start + b_len)]
        } else {
            Vec::new()
        };

        let ins_spans = if new_ins_start < new_ins_end {
            let b_start: usize = new_chars[..new_ins_start].iter().map(|c| c.len_utf8()).sum();
            let b_len: usize = new_chars[new_ins_start..new_ins_end].iter().map(|c| c.len_utf8()).sum();
            vec![(b_start, b_start + b_len)]
        } else {
            Vec::new()
        };

        (del_spans, ins_spans)
    }
}

/// Computes a Myers diff asynchronously on a background worker thread.
pub fn compute_diff_async<F>(old: String, new: String, on_complete: F)
where
    F: FnOnce(Vec<DiffHunk>) + Send + 'static,
{
    thread::spawn(move || {
        let hunks = MyersDiff::diff(&old, &new);
        on_complete(hunks);
    });
}

/// A paired side-by-side diff line containing optional left (deletion/context)
/// and right (insertion/context) diff lines.
#[derive(Debug, Clone, PartialEq)]
pub struct AlignedDiffLine<'a> {
    pub left: Option<&'a DiffLine>,
    pub right: Option<&'a DiffLine>,
    pub is_change: bool,
}

/// Pairs deletions and additions side-by-side, inserting spacer placeholders (`None`)
/// on the shorter side so that diff rows maintain horizontal alignment across panes.
pub fn zip_aligned_diff<'a>(removals: &[&'a DiffLine], additions: &[&'a DiffLine]) -> Vec<AlignedDiffLine<'a>> {
    let mut aligned = Vec::new();
    let max_len = usize::max(removals.len(), additions.len());

    for i in 0..max_len {
        aligned.push(AlignedDiffLine {
            left: removals.get(i).copied(),
            right: additions.get(i).copied(),
            is_change: true,
        });
    }
    aligned
}

/// DiffView widget supporting Unified and Side-by-Side Split layouts.
#[derive(Debug, Clone)]
pub struct DiffView {
    pub hunks: Vec<DiffHunk>,
    pub mode: DiffMode,
    pub scroll_y: usize,
    pub scroll_x: usize,
}

impl DiffView {
    pub fn new(old_text: &str, new_text: &str) -> Self {
        let hunks = MyersDiff::diff(old_text, new_text);
        Self {
            hunks,
            mode: DiffMode::Unified,
            scroll_y: 0,
            scroll_x: 0,
        }
    }

    pub fn with_mode(mut self, mode: DiffMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn set_mode(&mut self, mode: DiffMode) {
        self.mode = mode;
    }

    pub fn aligned_split_lines<'a>(hunks: &'a [DiffHunk]) -> Vec<AlignedDiffLine<'a>> {
        let mut aligned = Vec::new();

        for hunk in hunks {
            let mut removals = Vec::new();
            let mut additions = Vec::new();

            for line in &hunk.lines {
                match line.change {
                    ChangeType::Equal => {
                        if !removals.is_empty() || !additions.is_empty() {
                            aligned.extend(zip_aligned_diff(&removals, &additions));
                            removals.clear();
                            additions.clear();
                        }
                        aligned.push(AlignedDiffLine {
                            left: Some(line),
                            right: Some(line),
                            is_change: false,
                        });
                    }
                    ChangeType::Delete => {
                        removals.push(line);
                    }
                    ChangeType::Insert => {
                        additions.push(line);
                    }
                }
            }

            if !removals.is_empty() || !additions.is_empty() {
                aligned.extend(zip_aligned_diff(&removals, &additions));
            }
        }

        aligned
    }

    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>) {
        match self.mode {
            DiffMode::Unified => self.render_unified(canvas),
            DiffMode::Split => self.render_split(canvas),
        }
    }

    fn render_unified(&self, canvas: &mut CanvasSubviewMut<'_>) {
        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        if w == 0 || h == 0 {
            return;
        }

        let all_lines: Vec<&DiffLine> = self.hunks.iter().flat_map(|h| &h.lines).collect();
        let total = all_lines.len();

        for y in 0..h {
            let idx = self.scroll_y + y;
            let cy = y as u16;

            for cx in 0..canvas.width() {
                canvas.set_char(cx, cy, ' ', Color::Reset, Color::Reset, Modifier::empty());
            }

            if idx >= total {
                continue;
            }

            let dl = all_lines[idx];
            let (indicator, fg, base_bg, hl_bg) = match dl.change {
                ChangeType::Equal => (' ', Color::Rgb(205, 214, 244), Color::Reset, Color::Reset),
                ChangeType::Insert => (
                    '+',
                    Color::Rgb(166, 227, 161),
                    Color::Rgb(20, 45, 30),
                    Color::Rgb(40, 85, 50),
                ),
                ChangeType::Delete => (
                    '-',
                    Color::Rgb(243, 139, 168),
                    Color::Rgb(45, 20, 25),
                    Color::Rgb(85, 35, 45),
                ),
            };

            let old_str = dl
                .old_line_num
                .map(|n| format!("{:>4}", n))
                .unwrap_or_else(|| "    ".into());
            let new_str = dl
                .new_line_num
                .map(|n| format!("{:>4}", n))
                .unwrap_or_else(|| "    ".into());
            let gutter = format!("{} {} {} ", old_str, new_str, indicator);

            let mut col = 0;
            for ch in gutter.chars() {
                if col < w {
                    canvas.set_char(col as u16, cy, ch, Color::DarkGray, Color::Reset, Modifier::empty());
                    col += 1;
                }
            }

            for (byte_idx, ch) in dl.content.char_indices() {
                if col >= w {
                    break;
                }
                let is_highlighted = dl.intra_highlights.iter().any(|&(s, e)| byte_idx >= s && byte_idx < e);
                let bg = if is_highlighted { hl_bg } else { base_bg };
                canvas.set_char(col as u16, cy, ch, fg, bg, Modifier::empty());
                col += 1;
            }
        }
    }

    fn render_split(&self, canvas: &mut CanvasSubviewMut<'_>) {
        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        if w < 10 || h == 0 {
            return;
        }

        let half_w = (w.saturating_sub(1)) / 2;
        let sep_x = half_w as u16;

        let aligned_lines = Self::aligned_split_lines(&self.hunks);
        let total = aligned_lines.len();

        for y in 0..h {
            let idx = self.scroll_y + y;
            let cy = y as u16;

            for cx in 0..canvas.width() {
                canvas.set_char(cx, cy, ' ', Color::Reset, Color::Reset, Modifier::empty());
            }

            canvas.set_char(sep_x, cy, '│', Color::DarkGray, Color::Reset, Modifier::empty());

            if idx >= total {
                continue;
            }

            let pair = &aligned_lines[idx];

            if let Some(dl) = pair.left {
                let (indicator, fg) = match dl.change {
                    ChangeType::Delete => ('-', Color::Rgb(243, 139, 168)),
                    _ => (' ', Color::Rgb(205, 214, 244)),
                };
                let l_gutter = dl
                    .old_line_num
                    .map(|n| format!("{:>4}{}", n, indicator))
                    .unwrap_or_else(|| format!("    {}", indicator));
                let mut cx = 0;
                for ch in l_gutter.chars() {
                    if cx < half_w {
                        canvas.set_char(
                            cx as u16,
                            cy,
                            ch,
                            if pair.is_change { fg } else { Color::DarkGray },
                            Color::Reset,
                            Modifier::empty(),
                        );
                        cx += 1;
                    }
                }
                for (byte_idx, ch) in dl.content.char_indices() {
                    if cx < half_w {
                        let is_hl = dl.intra_highlights.iter().any(|&(s, e)| byte_idx >= s && byte_idx < e);
                        let bg = if pair.is_change {
                            if is_hl {
                                Color::Rgb(85, 35, 45)
                            } else {
                                Color::Rgb(45, 20, 25)
                            }
                        } else {
                            Color::Reset
                        };
                        canvas.set_char(cx as u16, cy, ch, fg, bg, Modifier::empty());
                        cx += 1;
                    }
                }
            }

            if let Some(dl) = pair.right {
                let (indicator, fg) = match dl.change {
                    ChangeType::Insert => ('+', Color::Rgb(166, 227, 161)),
                    _ => (' ', Color::Rgb(205, 214, 244)),
                };
                let r_gutter = dl
                    .new_line_num
                    .map(|n| format!("{:>4}{}", n, indicator))
                    .unwrap_or_else(|| format!("    {}", indicator));
                let mut rx = (sep_x + 1) as usize;
                for ch in r_gutter.chars() {
                    if rx < w {
                        canvas.set_char(
                            rx as u16,
                            cy,
                            ch,
                            if pair.is_change { fg } else { Color::DarkGray },
                            Color::Reset,
                            Modifier::empty(),
                        );
                        rx += 1;
                    }
                }
                for (byte_idx, ch) in dl.content.char_indices() {
                    if rx < w {
                        let is_hl = dl.intra_highlights.iter().any(|&(s, e)| byte_idx >= s && byte_idx < e);
                        let bg = if pair.is_change {
                            if is_hl {
                                Color::Rgb(40, 85, 50)
                            } else {
                                Color::Rgb(20, 45, 30)
                            }
                        } else {
                            Color::Reset
                        };
                        canvas.set_char(rx as u16, cy, ch, fg, bg, Modifier::empty());
                        rx += 1;
                    }
                }
            }
        }
    }
}
