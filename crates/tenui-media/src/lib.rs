#![forbid(unsafe_code)]
//! # Tenui Media (`tenui-media`)
//!
//! Real-time media, video streaming, and Kitty graphics protocol subsystem.
//!
//! `tenui-media` enables rich multimedia playback and image display within terminal emulators:
//!
//! ## Core Primitives
//!
//! - **[`HalfBlockVideoConverter`]**: Maps two vertical RGB pixels into a single character cell
//!   using the upper half block glyph (`▀`), where the upper pixel sets the cell foreground color
//!   and the lower pixel sets the cell background color. Includes area-averaged bicubic downscaling.
//! - **[`KittyGraphicsConverter`]**: Generates inline Kitty graphics protocol escape sequences
//!   (`\x1b_G...`), allowing full 24-bit unconstrained image rendering on terminals supporting the protocol.
//! - **[`MediaStreamController`]**: Frame rate governor providing dynamic backpressure, frame dropping,
//!   and temporal pacing under high terminal latency or slow socket throughput.
//! - **[`VideoFrame`]**: Decoded frame representation holding raw RGB/RGBA buffers, dimensions, and timestamps.
//!
//! ## Runnable Example: Half-Block Image Rendering
//!
//! ```rust
//! use tenui_core::{Buffer, Rect};
//! use tenui_media::HalfBlockVideoConverter;
//!
//! // 2x2 RGB image: Top row red, bottom row blue
//! let rgb_data = vec![
//!     255, 0, 0,   255, 0, 0,   // row 0: red, red
//!     0, 0, 255,   0, 0, 255,   // row 1: blue, blue
//! ];
//!
//! let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
//! let mut subview = buffer.subview_mut(Rect::new(0, 0, 2, 1));
//!
//! // Renders 2x2 pixels into 2x1 character cells
//! HalfBlockVideoConverter::render_rgb(&mut subview, 0, 0, 2, 2, &rgb_data);
//!
//! let cell = buffer.get(0, 0).unwrap();
//! assert_eq!(cell.symbol.as_str(), "▀");
//! ```

pub mod kitty;
pub mod video;

pub use kitty::{KittyGraphicsConverter, base64_encode};
pub use video::{HalfBlockVideoConverter, MediaStreamController, PixelFormat, VideoFrame};
