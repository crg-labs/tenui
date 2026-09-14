#![forbid(unsafe_code)]
//! # Tenui Sound (`tenui-sound`)
//!
//! Tactile audio feedback, earcons, and DEC terminal acoustics.
//!
//! `tenui-sound` provides auditory feedback for terminal user interfaces, generating
//! subtle acoustic cues for key presses, navigation transitions, and system events:
//!
//! ## Core Primitives
//!
//! - **[`SoundCue`]**: Semantic earcons including [`SoundCue::KeycapDepress`], [`SoundCue::KeycapRelease`],
//!   [`SoundCue::FocusMove`], [`SoundCue::ValidationFailure`], and [`SoundCue::TaskSuccess`].
//! - **[`TactileAudioEngine`]**: Audio dispatcher supporting both DEC terminal sound escape sequences
//!   (DECPS frequency/duration tone bursts) and terminal alert bells (`\x07`) with built-in temporal throttling
//!   to eliminate earcon saturation.
//!
//! ## Runnable Example
//!
//! ```rust
//! use tenui_sound::{SoundCue, TactileAudioEngine};
//! use std::io::Cursor;
//!
//! let mut engine = TactileAudioEngine::new().with_dec_audio(true);
//! let mut sink = Cursor::new(Vec::new());
//!
//! // Play a keypress acoustic click
//! engine.play_cue(SoundCue::KeycapDepress, &mut sink).unwrap();
//!
//! // Emitted DECPS escape tone
//! assert!(!sink.get_ref().is_empty());
//! ```

pub mod synth;

pub use synth::{SoundCue, TactileAudioEngine};
