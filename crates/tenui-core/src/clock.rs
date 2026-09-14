//! Precision frame clock and EWMA framerate smoothing.

use std::time::Instant;

/// Precision frame clock applying Exponentially Weighted Moving Average (EWMA)
/// to eliminate instantaneous framerate display jitter.
#[derive(Debug, Clone)]
pub struct FrameClock {
    last_frame_time: Instant,
    smoothed_fps: f32,
    active_handles: usize,
}

impl Default for FrameClock {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameClock {
    pub fn new() -> Self {
        Self {
            last_frame_time: Instant::now(),
            smoothed_fps: 60.0,
            active_handles: 0,
        }
    }

    /// Ticks the frame clock, measuring elapsed delta time and updating
    /// the smoothed FPS via EWMA filter (0.15 instant + 0.85 smoothed).
    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32().max(0.001);
        self.last_frame_time = now;

        let instant_fps = 1.0 / dt;
        self.smoothed_fps = 0.15 * instant_fps + 0.85 * self.smoothed_fps;
        self.smoothed_fps
    }

    /// Directly pushes an instantaneous delta for deterministic test simulations.
    pub fn tick_dt(&mut self, dt: f32) -> f32 {
        let dt_clamped = dt.max(0.0001);
        let instant_fps = 1.0 / dt_clamped;
        self.smoothed_fps = 0.15 * instant_fps + 0.85 * self.smoothed_fps;
        self.smoothed_fps
    }

    /// Returns the current smoothed framerate.
    pub fn fps(&self) -> f32 {
        self.smoothed_fps
    }

    /// Returns true if all particle lifetimes and active animation handles have settled.
    pub fn is_quiescent(&self) -> bool {
        self.active_handles == 0
    }

    pub fn register_active(&mut self) {
        self.active_handles += 1;
    }

    pub fn unregister_active(&mut self) {
        self.active_handles = self.active_handles.saturating_sub(1);
    }
}
