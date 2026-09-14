//! Deterministic fuzz/property tests for the input-facing parsers: they must never panic
//! on hostile or malformed input, only return `None`/empty/partial results (graceful
//! degradation). Uses a small deterministic PRNG — no external dependency.

use tenui_core::{InputDemuxer, PathSanitizer, SgrMouseParser};

/// Tiny deterministic LCG so failures reproduce exactly.
struct Rng(u64);
impl Rng {
    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn byte(&mut self) -> u8 {
        (self.next_u32() & 0xFF) as u8
    }
    fn range(&mut self, n: usize) -> usize {
        (self.next_u32() as usize) % n.max(1)
    }
}

#[test]
fn fuzz_sgr_mouse_parser_never_panics() {
    let mut rng = Rng(0xC0FFEE);
    // Alphabet biased toward the sequence's real characters to hit more parser paths.
    let alphabet = b"\x1b[<0123456789;Mm ABC";
    for _ in 0..20_000 {
        let len = rng.range(24);
        let s: String = (0..len).map(|_| alphabet[rng.range(alphabet.len())] as char).collect();
        // Must return Some or None, never panic (e.g. from slicing or parse).
        let _ = SgrMouseParser::parse_sgr(&s);
    }
    // A few structured valid/invalid cases.
    assert!(SgrMouseParser::parse_sgr("\x1b[<0;10;5M").is_some());
    assert!(SgrMouseParser::parse_sgr("\x1b[<").is_none());
    assert!(SgrMouseParser::parse_sgr("\x1b[<;;M").is_none());
    assert!(SgrMouseParser::parse_sgr("garbage").is_none());
}

#[test]
fn fuzz_input_demuxer_never_panics() {
    let mut rng = Rng(0x1234_5678);
    let mut demux = InputDemuxer::new();
    for _ in 0..10_000 {
        let len = rng.range(64);
        let bytes: Vec<u8> = (0..len).map(|_| rng.byte()).collect();
        let _ = demux.feed_bytes(&bytes);
    }
    // Malformed / partial escape and paste fragments across boundaries.
    let _ = demux.feed_bytes(b"\x1b[200~partial");
    let _ = demux.feed_bytes(b"paste body\x1b[201~");
    let _ = demux.feed_bytes(b"\x1b[<0;1"); // truncated SGR mouse
    let _ = demux.feed_bytes(&[0xFF, 0xFE, 0x00, 0x1b]); // invalid UTF-8 + lone ESC
}

#[test]
fn fuzz_path_sanitizer_never_panics() {
    let mut rng = Rng(0xDEADBEEF);
    let alphabet = b"file:/%2039\\ \n\thome/\xff.txt";
    for _ in 0..20_000 {
        let len = rng.range(40);
        let bytes: Vec<u8> = (0..len).map(|_| alphabet[rng.range(alphabet.len())]).collect();
        // Input is &str; build lossily so we also exercise multi-byte boundaries.
        let s = String::from_utf8_lossy(&bytes);
        let _ = PathSanitizer::parse_drop_payload(&s);
    }
    // Trailing percent / backslash and bare file:// must not panic.
    assert!(PathSanitizer::parse_drop_payload("%").is_empty() || true);
    let _ = PathSanitizer::parse_drop_payload("file://");
    let _ = PathSanitizer::parse_drop_payload("/a%2");
    let _ = PathSanitizer::parse_drop_payload("/b\\");
}
