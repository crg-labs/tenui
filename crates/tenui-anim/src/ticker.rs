use std::{
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

/// Dynamic occupancy scheduling ticker for kinetic animations.
///
/// Operates as a demand-driven state machine governed by an atomic counter N_active:
/// - When N_active == 0: sleep / yield execution, zero-idle CPU overhead.
/// - When N_active > 0: 16.6ms (60 FPS) high-resolution evaluation tick.
/// - Delta time is clamped to [0, 50ms] to prevent temporal explosion.
pub struct ActiveTicker {
    n_active: AtomicUsize,
    last_tick: Mutex<Option<Instant>>,
}

impl Default for ActiveTicker {
    fn default() -> Self {
        Self::new()
    }
}

impl ActiveTicker {
    pub const TARGET_FRAME_DURATION: Duration = Duration::from_micros(16_667); // 60 FPS (~16.6ms)
    pub const MAX_DELTA_TIME_SECS: f32 = 0.050; // 50ms clamp

    pub fn new() -> Self {
        Self {
            n_active: AtomicUsize::new(0),
            last_tick: Mutex::new(None),
        }
    }

    /// Registers a new active animation handle, waking the ticker if transitioning from 0 -> 1.
    pub fn register_active(&self) -> usize {
        let prev = self.n_active.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            *self.last_tick.lock().unwrap() = Some(Instant::now());
        }
        prev + 1
    }

    /// Unregisters a settled animation handle. When count drops to 0, ticker enters idle sleep.
    pub fn unregister_active(&self) -> usize {
        let prev = self.n_active.load(Ordering::SeqCst);
        if prev == 0 {
            return 0;
        }
        let prev = self.n_active.fetch_sub(1, Ordering::SeqCst);
        let curr = prev.saturating_sub(1);
        if curr == 0 {
            *self.last_tick.lock().unwrap() = None;
        }
        curr
    }

    pub fn active_count(&self) -> usize {
        self.n_active.load(Ordering::SeqCst)
    }

    pub fn is_active(&self) -> bool {
        self.active_count() > 0
    }

    /// Advances the ticker and computes clamped delta time (dt in seconds).
    /// Returns `None` if no animations are currently active (zero-idle sleep state).
    pub fn tick(&self) -> Option<f32> {
        if !self.is_active() {
            return None;
        }

        let mut lock = self.last_tick.lock().unwrap();
        let now = Instant::now();
        let dt = match *lock {
            Some(last) => {
                let elapsed = now.duration_since(last).as_secs_f32();
                elapsed.clamp(0.0, Self::MAX_DELTA_TIME_SECS)
            }
            None => 0.016667,
        };
        *lock = Some(now);
        Some(dt)
    }
}
