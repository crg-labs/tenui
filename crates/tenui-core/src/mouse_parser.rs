//! Low-level ANSI SGR 1006 mouse escape sequence parser.
//!
//! Coordinates are decoded in cell units (SGR 1006). Sub-cell pixel coordinates
//! (SGR 1016) are handled separately by [`crate::input::PixelDecoupler`].
//!
//! Button-held motion is emitted as [`MouseEvent::Move`]; [`MouseEvent::Drag`] is
//! synthesized downstream by [`crate::mouse_dispatcher::MouseDispatcher`], which owns
//! the press-state needed to attach the originating button and start point.

use crate::{
    geometry::Point,
    mouse::{MouseButton, MouseEvent},
};

/// Parser for ANSI SGR 1006 (`\x1b[<B;X;YM` / `\x1b[<B;X;Ym`) mouse escape sequences.
pub struct SgrMouseParser;

impl SgrMouseParser {
    /// Parses an ANSI SGR 1006 escape sequence: `\x1b[<B;X;YM` or `\x1b[<B;X;Ym`
    pub fn parse_sgr(payload: &str) -> Option<MouseEvent> {
        let trimmed = payload.strip_prefix("\x1b[<")?;
        let is_release = trimmed.ends_with('m');
        let is_press_or_motion = trimmed.ends_with('M');
        if !is_release && !is_press_or_motion {
            return None;
        }

        let body = &trimmed[..trimmed.len() - 1];
        let mut parts = body.split(';');
        let b: u16 = parts.next()?.parse().ok()?;
        let x: u16 = parts.next()?.parse().ok()?;
        let y: u16 = parts.next()?.parse().ok()?;

        // SGR uses 1-based coordinates
        let position = Point::new(x.saturating_sub(1), y.saturating_sub(1));

        let is_motion = (b & 32) != 0;
        let is_wheel = (b & 64) != 0;

        if is_wheel {
            return match b & 3 {
                0 => Some(MouseEvent::Scroll {
                    delta_x: 0.0,
                    delta_y: -1.0,
                    is_pixel: false,
                }),
                1 => Some(MouseEvent::Scroll {
                    delta_x: 0.0,
                    delta_y: 1.0,
                    is_pixel: false,
                }),
                2 => Some(MouseEvent::Scroll {
                    delta_x: -1.0,
                    delta_y: 0.0,
                    is_pixel: false,
                }),
                3 => Some(MouseEvent::Scroll {
                    delta_x: 1.0,
                    delta_y: 0.0,
                    is_pixel: false,
                }),
                _ => None,
            };
        }

        let button = match b & 3 {
            0 => MouseButton::Left,
            1 => MouseButton::Middle,
            2 => MouseButton::Right,
            _ => MouseButton::Left,
        };

        if is_motion {
            Some(MouseEvent::Move { position })
        } else if is_release {
            Some(MouseEvent::Up { button, position })
        } else {
            Some(MouseEvent::Down { button, position })
        }
    }
}
