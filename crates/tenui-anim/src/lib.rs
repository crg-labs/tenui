#![forbid(unsafe_code)]
//! # Tenui Animation & Kinetics (`tenui-anim`)
//!
//! Sub-cell kinetics, closed-form analytical spring dynamics, and demand-driven frame scheduling.
//!
//! `tenui-anim` brings smooth physical motion to character-cell terminals through three
//! core primitives:
//!
//! - **[`AnalyticalSpring`]**: Closed-form analytical solver for damped harmonic oscillators.
//!   Unlike numerical Euler or Verlet integrators, closed-form evaluation avoids numerical
//!   drift, behaves identically across variable frame rates, and guarantees smooth interruption
//!   invariance when targets change mid-flight.
//! - **[`SpringConfig`]**: Physical spring tuning parameters with pre-configured damping regimes:
//!   [`SpringConfig::CRITICAL`] (fastest convergence with zero overshoot),
//!   [`SpringConfig::BOUNCY`] (under-damped spring oscillation), and
//!   [`SpringConfig::GENTLE`] (over-damped exponential return).
//! - **[`MicroStepper`]**: Maps continuous floating-point layout coordinates into 1/8th Unicode
//!   fractional character glyphs (e.g. `▏`, `▎`, `▍`, `▌`, `▋`, `▊`, `▉`), unlocking sub-cell
//!   spatial smoothness.
//! - **[`ActiveTicker`]**: Zero-idle power manager. Tracks active animations and transitions into
//!   a quiescent sleep state when all springs settle ($N_{\text{active}} = 0$), preventing battery
//!   drain when the UI is static.
//!
//! ## Runnable Example
//!
//! ```rust
//! use tenui_anim::{AnalyticalSpring, SpringConfig, MicroStepper};
//!
//! // Create a spring starting at 0.0 with critical damping
//! let mut spring = AnalyticalSpring::new(0.0, SpringConfig::CRITICAL);
//!
//! // Set a new target position
//! spring.set_target(10.0);
//!
//! // Advance the simulation forward by 100ms
//! let (pos, _) = spring.advance(0.100);
//! assert!(pos > 0.0 && pos < 10.0);
//!
//! // Resolve continuous position into whole cells and fractional sub-cell glyphs
//! let sub_cell = MicroStepper::resolve_horizontal(pos);
//! assert!(sub_cell.whole_cells < 10);
//! ```

pub mod spring;
pub mod stepper;
pub mod ticker;

pub use spring::{AnalyticalSpring, SpringConfig};
pub use stepper::{MicroStepCell, MicroStepper};
pub use ticker::ActiveTicker;
