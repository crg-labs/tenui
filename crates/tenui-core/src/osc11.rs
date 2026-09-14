//! Dynamic Terminal Palette Interrogation (OSC 11).

/// Returns the OSC 11 ambient background interrogation escape query.
pub fn osc11_query() -> &'static str {
    "\x1b]11;?\x07"
}

/// Parses an OSC 11 response from the terminal (e.g. `\x1b]11;rgb:rrrr/gggg/bbbb\x07`).
///
/// Supports 2-digit, 3-digit, and 4-digit hex channel formats.
pub fn parse_osc11_response(resp: &str) -> Option<(u8, u8, u8)> {
    let rgb_marker = "rgb:";
    let start_idx = resp.find(rgb_marker)? + rgb_marker.len();
    let end_idx = resp[start_idx..]
        .find(['\x07', '\x1b', ';', '\n'])
        .map(|idx| start_idx + idx)
        .unwrap_or(resp.len());

    let payload = &resp[start_idx..end_idx];
    let channels: Vec<&str> = payload.split('/').collect();
    if channels.len() != 3 {
        return None;
    }

    let parse_channel = |ch: &str| -> Option<u8> {
        let ch = ch.trim();
        if ch.len() >= 2 {
            u8::from_str_radix(&ch[0..2], 16).ok()
        } else if ch.len() == 1 {
            u8::from_str_radix(ch, 16).map(|v| v * 17).ok()
        } else {
            None
        }
    };

    Some((
        parse_channel(channels[0])?,
        parse_channel(channels[1])?,
        parse_channel(channels[2])?,
    ))
}

/// Computes relative luminance of an RGB background using ITU-R BT.709 coefficients:
/// Y = 0.2126 * R + 0.7152 * G + 0.0722 * B
pub fn relative_luminance(r: u8, g: u8, b: u8) -> f32 {
    let r_norm = r as f32 / 255.0;
    let g_norm = g as f32 / 255.0;
    let b_norm = b as f32 / 255.0;
    0.2126 * r_norm + 0.7152 * g_norm + 0.0722 * b_norm
}

/// Determines whether the host terminal ambient palette is in Light Mode (Y > 0.5).
pub fn is_light_theme(r: u8, g: u8, b: u8) -> bool {
    relative_luminance(r, g, b) > 0.5
}
