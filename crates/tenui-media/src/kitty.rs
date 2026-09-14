//! Kitty Graphics Protocol hardware converter and base64 encoder.
//!
//! Provides direct GPU-accelerated texture blitting for modern terminal emulators
//! supporting the Kitty APC Graphics Protocol (Kitty, Ghostty, WezTerm).

/// Encodes byte slices into standard RFC 4648 Base64 strings.
#[allow(clippy::chunks_exact_to_as_chunks)]
pub fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    let mut chunks = data.chunks_exact(3);
    for chunk in chunks.by_ref() {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        out.push(TABLE[(n & 0x3F) as usize] as char);
    }
    let rem = chunks.remainder();
    if rem.len() == 1 {
        let n = (rem[0] as u32) << 16;
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem.len() == 2 {
        let n = ((rem[0] as u32) << 16) | ((rem[1] as u32) << 8);
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        out.push('=');
    }
    out
}

/// Converter generating Kitty APC graphics escape sequences for hardware texture blitting.
pub struct KittyGraphicsConverter;

impl KittyGraphicsConverter {
    /// Formats an RGB24 frame into chunked Kitty APC escape codes targeting a specific (x, y) cell origin
    /// and scaled across `cols` columns and `rows` rows.
    #[allow(clippy::too_many_arguments)]
    pub fn format_rgb_stream(
        origin_x: u16,
        origin_y: u16,
        cols: u16,
        rows: u16,
        img_w: u16,
        img_h: u16,
        rgb: &[u8],
        image_id: u32,
    ) -> String {
        if cols == 0 || rows == 0 || img_w == 0 || img_h == 0 || rgb.is_empty() {
            return String::new();
        }

        let b64 = base64_encode(rgb);
        let b64_bytes = b64.as_bytes();
        let chunk_size = 4096;
        let mut out = String::with_capacity(b64.len() + 512);

        // 1. Position cursor at destination origin (1-indexed row and col)
        out.push_str(&format!("\x1b[{};{}H", origin_y + 1, origin_x + 1));

        // 2. Transmit chunks (Kitty limits chunks to 4096 bytes each)
        if b64_bytes.len() <= chunk_size {
            out.push_str(&format!(
                "\x1b_Ga=T,f=24,s={},v={},c={},r={},i={},q=2,C=1,m=0;{}\x1b\\",
                img_w, img_h, cols, rows, image_id, b64
            ));
        } else {
            let mut offset = 0;
            let mut first = true;
            while offset < b64_bytes.len() {
                let end = (offset + chunk_size).min(b64_bytes.len());
                let chunk_str = &b64[offset..end];
                let is_last = end == b64_bytes.len();
                let m = if is_last { 0 } else { 1 };

                if first {
                    out.push_str(&format!(
                        "\x1b_Ga=T,f=24,s={},v={},c={},r={},i={},q=2,C=1,m={};{}\x1b\\",
                        img_w, img_h, cols, rows, image_id, m, chunk_str
                    ));
                    first = false;
                } else {
                    out.push_str(&format!("\x1b_Gm={};{}\x1b\\", m, chunk_str));
                }
                offset = end;
            }
        }
        out
    }

    /// Generates escape sequence to delete a specific Kitty graphics image by ID.
    pub fn delete_image_apc(image_id: u32) -> String {
        format!("\x1b_Ga=d,d=i,i={},q=2\x1b\\", image_id)
    }

    /// Generates escape sequence to delete all visible Kitty graphics images.
    pub fn delete_all_apc() -> String {
        "\x1b_Ga=d,d=A,q=2\x1b\\".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn test_kitty_format_single_chunk() {
        let rgb = [255u8, 0, 0, 0, 255, 0];
        let seq = KittyGraphicsConverter::format_rgb_stream(10, 5, 20, 10, 2, 1, &rgb, 42);
        assert!(seq.contains("\x1b[6;11H"));
        assert!(seq.contains("\x1b_Ga=T,f=24,s=2,v=1,c=20,r=10,i=42,q=2,C=1,m=0;"));
        assert!(seq.ends_with("\x1b\\"));
    }

    #[test]
    fn test_kitty_deletion_escape_codes() {
        assert_eq!(
            KittyGraphicsConverter::delete_image_apc(42),
            "\x1b_Ga=d,d=i,i=42,q=2\x1b\\"
        );
        assert_eq!(KittyGraphicsConverter::delete_all_apc(), "\x1b_Ga=d,d=A,q=2\x1b\\");
    }
}
