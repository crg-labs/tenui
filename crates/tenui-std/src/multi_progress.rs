use tenui_core::{CanvasSubviewMut, Color, Modifier};

use crate::theme::readable_on;

const FRACTIONAL_BLOCKS: [char; 9] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

/// Execution state of a tracked asynchronous task slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

/// A single dynamic task slot in MultiProgress.
#[derive(Clone, Debug)]
pub struct TaskSlot {
    pub id: usize,
    pub label: String,
    pub progress: f32, // 0.0 to 1.0
    pub status: TaskStatus,
    pub rate_bytes_per_sec: f32,
    pub eta_secs: Option<f32>,
    /// Analytical spring height factor for dynamic collapse [1.0 -> 0.0]
    pub collapse_factor: f32,
}

impl TaskSlot {
    pub fn new(id: usize, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            progress: 0.0,
            status: TaskStatus::Running,
            rate_bytes_per_sec: 0.0,
            eta_secs: None,
            collapse_factor: 1.0,
        }
    }

    /// Updates throughput using an Exponentially Weighted Moving Average (EWMA).
    pub fn update_rate_ewma(&mut self, new_rate: f32, alpha: f32) {
        let a = alpha.clamp(0.01, 1.0);
        self.rate_bytes_per_sec = a * new_rate + (1.0 - a) * self.rate_bytes_per_sec;
    }
}

/// Dynamic task progress tracker visualizing concurrent operations.
pub struct MultiProgress {
    pub tasks: Vec<TaskSlot>,
    pub next_id: usize,
    pub auto_reclaim_completed: bool,
}

impl MultiProgress {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            next_id: 1,
            auto_reclaim_completed: true,
        }
    }

    pub fn add_task(&mut self, label: impl Into<String>) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.tasks.push(TaskSlot::new(id, label));
        id
    }

    pub fn get_task_mut(&mut self, id: usize) -> Option<&mut TaskSlot> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    /// Advances spring collapse animations for completed tasks and purges fully collapsed slots.
    pub fn tick_spring_collapse(&mut self, dt: f32) {
        if !self.auto_reclaim_completed {
            return;
        }

        for task in self.tasks.iter_mut() {
            if task.status == TaskStatus::Completed {
                // Smooth collapse toward 0.0
                task.collapse_factor = (task.collapse_factor - dt * 2.0).max(0.0);
            }
        }

        // Purge tasks whose height has collapsed to 0.0
        self.tasks.retain(|t| t.collapse_factor > 0.01);
    }

    /// Renders active tasks with fractional micro-stepping progress bars.
    pub fn render(&self, surface: &mut CanvasSubviewMut<'_>, fg: Color, bg: Color) {
        let w = surface.width();
        let h = surface.height();
        if w < 20 || h == 0 || self.tasks.is_empty() {
            return;
        }

        for (row, task) in self.tasks.iter().take(h as usize).enumerate() {
            let y = row as u16;

            // Status icon
            let (icon, icon_color) = match task.status {
                TaskStatus::Queued => ("○", Color::DarkGray),
                TaskStatus::Running => ("⠸", Color::CYAN),
                TaskStatus::Completed => ("✓", Color::GREEN),
                TaskStatus::Failed => ("✗", Color::RED),
            };
            surface.write_str_clipped(0, y, icon, icon_color, bg);

            // Label
            let label_w = 26u16.min(w / 3);
            let truncated_label = if task.label.len() > label_w as usize {
                &task.label[..label_w as usize]
            } else {
                &task.label
            };
            surface.write_str_clipped(2, y, truncated_label, fg, bg);

            // Fractional Progress Bar
            let bar_start = label_w + 4;
            let bar_w = 20u16.min(w.saturating_sub(bar_start + 18));
            if bar_w > 4 {
                let total_eighths = (task.progress.clamp(0.0, 1.0) * (bar_w as f32 * 8.0)).round() as usize;
                let full_blocks = total_eighths / 8;
                let remainder = total_eighths % 8;

                surface.set_char(bar_start, y, '[', Color::DarkGray, bg, Modifier::empty());
                for bx in 0..bar_w {
                    let ch = if (bx as usize) < full_blocks {
                        FRACTIONAL_BLOCKS[8]
                    } else if (bx as usize) == full_blocks && remainder > 0 {
                        FRACTIONAL_BLOCKS[remainder]
                    } else {
                        '░'
                    };
                    let bar_color = if task.status == TaskStatus::Completed {
                        Color::GREEN
                    } else {
                        Color::CYAN
                    };
                    surface.set_char(bar_start + 1 + bx, y, ch, bar_color, bg, Modifier::empty());
                }
                surface.set_char(bar_start + 1 + bar_w, y, ']', Color::DarkGray, bg, Modifier::empty());
            }

            // Percentage & Rate
            let stats = format!(
                "{:5.1}% ({:.1} MB/s)",
                task.progress * 100.0,
                task.rate_bytes_per_sec / (1024.0 * 1024.0)
            );
            let stats_x = w.saturating_sub(stats.len() as u16);
            surface.write_str_clipped(stats_x, y, &stats, readable_on(bg), bg);
        }
    }
}

impl Default for MultiProgress {
    fn default() -> Self {
        Self::new()
    }
}
