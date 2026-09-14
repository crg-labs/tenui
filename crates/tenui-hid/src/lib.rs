#![forbid(unsafe_code)]
//! # Tenui HID (`tenui-hid`)
//!
//! Universal Hardware HID & Peripheral Controller Bridge subsystem.
//!
//! `tenui-hid` bridges physical control surfaces (rotary encoders, motorized sliders,
//! macro keypads, and MIDI controllers) to continuous kinetic animation momentum and reactive UI signals:
//!
//! ## Core Primitives
//!
//! - **[`HardwareAction`]**: Normalized event enum representing slider movements, rotary dial impulses,
//!   and physical button presses.
//! - **[`RotaryImpulseController`]**: Translates discrete rotary encoder detent clicks into continuous
//!   kinetic momentum ($\Delta v = k \cdot v_{\text{dial}} \cdot \text{ticks}$) with exponential friction decay.
//! - **[`HidEventReactor`]**: Dispatches and routes incoming hardware events to bound application actions.
//! - **[`KeyLedFeedback`]**: Manages RGB LED state feedback on programmable macro pads.
//! - **Normalization Helpers**: [`normalize_midi_7bit`], [`normalize_midi_14bit`], [`normalize_hid_16bit`].
//!
//! ## Runnable Example: Kinetic Rotary Dial Tracking
//!
//! ```rust
//! use tenui_hid::RotaryImpulseController;
//!
//! let mut controller = RotaryImpulseController::new(1.5);
//!
//! // Physical dial turns 3 clicks clockwise at high speed
//! let impulse = controller.translate_rotary_to_kinetic_impulse(3, 2.0);
//! assert!(impulse > 0.0);
//!
//! // Step simulation forward by 16ms
//! let delta_pos = controller.step(0.016);
//! assert!(delta_pos > 0.0);
//! ```

pub mod controller;

pub use controller::{
    HardwareAction, HidEventReactor, KeyLedFeedback, RotaryImpulseController, normalize_hid_16bit, normalize_midi_7bit,
    normalize_midi_14bit,
};
