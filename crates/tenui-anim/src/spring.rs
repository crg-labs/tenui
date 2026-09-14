use std::f32;

/// Configuration parameters for analytical damped harmonic oscillator springs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringConfig {
    /// Spring stiffness coefficient (k).
    pub stiffness: f32,
    /// Damping coefficient (c).
    pub damping: f32,
    /// Equilibrium convergence threshold (epsilon).
    pub precision: f32,
}

impl SpringConfig {
    /// Regime I: Critical Damping (zeta = 1.0) — fastest rest with zero bounce.
    pub const CRITICAL: Self = Self {
        stiffness: 180.0,
        damping: 26.832815, // c = 2 * sqrt(180)
        precision: 0.005,
    };

    /// Regime II: Under-Damping (zeta < 1.0) — oscillatory spring bounce.
    pub const BOUNCY: Self = Self {
        stiffness: 220.0,
        damping: 15.0,
        precision: 0.005,
    };

    /// Regime III: Over-Damping (zeta > 1.0) — slow monotonic exponential return.
    pub const GENTLE: Self = Self {
        stiffness: 100.0,
        damping: 25.0,
        precision: 0.005,
    };
}

/// Closed-form analytical spring dynamics engine.
///
/// Solves the continuous differential equation m*x'' + c*x' + k*x = 0 analytically,
/// ensuring zero floating-point accumulation drift and interruption invariance.
#[derive(Debug, Clone)]
pub struct AnalyticalSpring {
    config: SpringConfig,
    x_target: f32,
    x0: f32,
    v0: f32,
    elapsed_time: f32,
    omega0: f32,
    zeta: f32,
    is_settled: bool,
}

impl AnalyticalSpring {
    pub fn new(initial: f32, config: SpringConfig) -> Self {
        let omega0 = config.stiffness.sqrt();
        let zeta = config.damping / (2.0 * omega0);

        Self {
            config,
            x_target: initial,
            x0: 0.0,
            v0: 0.0,
            elapsed_time: 0.0,
            omega0,
            zeta,
            is_settled: true,
        }
    }

    /// Sets a new target, preserving existing position and momentum (Interruption Invariance).
    pub fn set_target(&mut self, target: f32) {
        if (target - self.x_target).abs() < self.config.precision && self.is_settled {
            return;
        }

        let (current_x, current_v) = if self.is_settled {
            (self.x_target, 0.0)
        } else {
            self.sample(self.elapsed_time)
        };

        self.x_target = target;
        self.x0 = current_x - target;
        self.v0 = current_v;
        self.elapsed_time = 0.0;
        self.is_settled = false;
    }

    /// Injects instantaneous momentum/velocity (e.g. trackpad flick gesture).
    pub fn inject_velocity(&mut self, v: f32) {
        let (current_x, current_v) = if self.is_settled {
            (self.x_target, 0.0)
        } else {
            self.sample(self.elapsed_time)
        };

        self.x0 = current_x - self.x_target;
        self.v0 = current_v + v;
        self.elapsed_time = 0.0;
        self.is_settled = false;
    }

    /// Advances the spring simulation by `dt` seconds, returning `(current_value, is_settled)`.
    ///
    /// Honors the global accessibility "reduce motion" preference
    /// ([`tenui_core::a11y::reduce_motion`]): when enabled, the spring snaps straight to
    /// its target rather than animating.
    pub fn advance(&mut self, dt: f32) -> (f32, bool) {
        if self.is_settled {
            return (self.x_target, true);
        }

        if tenui_core::a11y::reduce_motion() {
            self.is_settled = true;
            return (self.x_target, true);
        }

        self.elapsed_time += dt;
        let (x, v) = self.sample(self.elapsed_time);

        if (x - self.x_target).abs() < self.config.precision && v.abs() < self.config.precision {
            self.is_settled = true;
            (self.x_target, true)
        } else {
            (x, false)
        }
    }

    /// Exact analytical solution sampling at arbitrary time `t`.
    pub fn sample(&self, t: f32) -> (f32, f32) {
        if (self.zeta - 1.0).abs() < 1e-4 {
            // Regime I: Critical Damping
            let c1 = self.x0;
            let c2 = self.v0 + self.omega0 * self.x0;
            let decay = (-self.omega0 * t).exp();

            let x = self.x_target + (c1 + c2 * t) * decay;
            let v = (c2 - self.omega0 * (c1 + c2 * t)) * decay;
            (x, v)
        } else if self.zeta < 1.0 {
            // Regime II: Under-Damping
            let alpha = self.zeta * self.omega0;
            let omega_d = self.omega0 * (1.0 - self.zeta * self.zeta).sqrt();
            let decay = (-alpha * t).exp();

            let c1 = self.x0;
            let c2 = (self.v0 + alpha * self.x0) / omega_d;

            let cos_d = (omega_d * t).cos();
            let sin_d = (omega_d * t).sin();

            let x = self.x_target + decay * (c1 * cos_d + c2 * sin_d);
            let v = decay * ((c2 * omega_d - c1 * alpha) * cos_d - (c1 * omega_d + c2 * alpha) * sin_d);
            (x, v)
        } else {
            // Regime III: Over-Damping
            let root = (self.zeta * self.zeta - 1.0).sqrt();
            let gamma1 = -self.omega0 * (self.zeta - root);
            let gamma2 = -self.omega0 * (self.zeta + root);

            let c2 = (self.v0 - gamma1 * self.x0) / (gamma2 - gamma1);
            let c1 = self.x0 - c2;

            let e1 = (gamma1 * t).exp();
            let e2 = (gamma2 * t).exp();

            let x = self.x_target + c1 * e1 + c2 * e2;
            let v = c1 * gamma1 * e1 + c2 * gamma2 * e2;
            (x, v)
        }
    }

    pub fn value(&self) -> f32 {
        if self.is_settled {
            self.x_target
        } else {
            self.sample(self.elapsed_time).0
        }
    }

    pub fn is_settled(&self) -> bool {
        self.is_settled
    }

    pub fn target(&self) -> f32 {
        self.x_target
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advance_honors_reduce_motion() {
        // Isolated in the lib unit-test binary; integration spring tests live in
        // tests/anim_tests.rs (a separate process) where the global stays default-false.
        let mut spring = AnalyticalSpring::new(0.0, SpringConfig::BOUNCY);
        spring.set_target(100.0);

        tenui_core::a11y::set_reduce_motion(true);
        let (value, settled) = spring.advance(0.016);
        tenui_core::a11y::set_reduce_motion(false);

        assert!(settled, "reduce-motion must settle immediately");
        assert_eq!(value, 100.0, "spring snaps straight to target");
    }
}
