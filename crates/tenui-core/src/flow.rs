//! Network-Aware Adaptive Degradation (The SSH Flow Controller).

/// Connection bandwidth state derived from write acknowledgement latency (tau_ack).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkBandwidthState {
    /// Nominal (tau < 16ms): 60 FPS target, 24-bit TrueColor RGB, sub-cell micro-steps.
    Nominal,
    /// Degraded (16ms <= tau < 75ms): throttled to 30 FPS, ANSI 256 downsampling, integer cell snapping.
    Degraded,
    /// Congested (tau >= 75ms): clamped to 15 FPS, ANSI 16 CIELAB downsampling, drop intermediate frames.
    Congested,
}

/// Color downsampling tier corresponding to network bandwidth state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorTier {
    TrueColor,
    Ansi256,
    Ansi16,
}

/// SSH network flow controller dynamically adapting rendering fidelity to socket latency.
pub struct SshFlowController {
    current_latency_ms: f32,
    state: NetworkBandwidthState,
}

impl Default for SshFlowController {
    fn default() -> Self {
        Self {
            current_latency_ms: 0.0,
            state: NetworkBandwidthState::Nominal,
        }
    }
}

impl SshFlowController {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a write completion round-trip delta (latency in ms) and transitions state.
    pub fn record_latency(&mut self, latency_ms: f32) -> NetworkBandwidthState {
        self.current_latency_ms = latency_ms;
        self.state = if latency_ms < 16.0 {
            NetworkBandwidthState::Nominal
        } else if latency_ms < 75.0 {
            NetworkBandwidthState::Degraded
        } else {
            NetworkBandwidthState::Congested
        };
        self.state
    }

    pub fn state(&self) -> NetworkBandwidthState {
        self.state
    }

    pub fn current_latency_ms(&self) -> f32 {
        self.current_latency_ms
    }

    /// Target frame rate in frames per second.
    pub fn target_fps(&self) -> u32 {
        match self.state {
            NetworkBandwidthState::Nominal => 60,
            NetworkBandwidthState::Degraded => 30,
            NetworkBandwidthState::Congested => 15,
        }
    }

    /// Returns whether sub-cell fractional micro-stepping is permitted in the current state.
    pub fn should_microstep(&self) -> bool {
        self.state == NetworkBandwidthState::Nominal
    }

    /// Returns whether intermediate animation diff frames should be dropped.
    pub fn should_drop_intermediate_frames(&self) -> bool {
        self.state == NetworkBandwidthState::Congested
    }

    /// Color downsampling fidelity tier.
    pub fn color_tier(&self) -> ColorTier {
        match self.state {
            NetworkBandwidthState::Nominal => ColorTier::TrueColor,
            NetworkBandwidthState::Degraded => ColorTier::Ansi256,
            NetworkBandwidthState::Congested => ColorTier::Ansi16,
        }
    }
}
