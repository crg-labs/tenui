/// Deterministic stream fuzzer and chaos injection engine for terminal byte streams and event loops.
#[derive(Debug, Clone)]
pub struct ChaosStreamFuzzer {
    pub drop_rate: f32,
    pub fragment_multibyte: bool,
}

impl Default for ChaosStreamFuzzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ChaosStreamFuzzer {
    /// Creates a fuzzer with default settings (5% drop rate, aggressive 1-byte multibyte fragmentation).
    pub fn new() -> Self {
        Self {
            drop_rate: 0.05,
            fragment_multibyte: true,
        }
    }

    pub fn with_drop_rate(mut self, rate: f32) -> Self {
        self.drop_rate = rate.clamp(0.0, 1.0);
        self
    }

    pub fn with_fragment_multibyte(mut self, fragment: bool) -> Self {
        self.fragment_multibyte = fragment;
        self
    }

    /// Fragments a valid UTF-8 and ANSI byte sequence into disrupted micro-chunks.
    pub fn fuzz_payload(&self, input: &[u8]) -> Vec<Vec<u8>> {
        let mut chunks = Vec::new();
        let mut i = 0;

        while i < input.len() {
            // Slices into micro-packets (1 to 3 bytes if fragmenting, else 4 bytes)
            let step = if self.fragment_multibyte {
                // Alternating deterministic packet sizes (1, 2, 3) based on index
                (i % 3) + 1
            } else {
                4
            };
            let end = (i + step).min(input.len());
            chunks.push(input[i..end].to_vec());
            i = end;
        }

        chunks
    }

    /// Fragments input into fixed chunk sizes.
    pub fn fragment_into_chunks(&self, input: &[u8], chunk_size: usize) -> Vec<Vec<u8>> {
        let size = chunk_size.max(1);
        input.chunks(size).map(|c| c.to_vec()).collect()
    }

    /// Injects truncated CSI / OSC sequences into a stream without closing terminators.
    pub fn inject_malformed_ansi(&self, input: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(input.len() + 64);
        let mid = input.len() / 2;
        out.extend_from_slice(&input[..mid]);

        // Inject truncated truecolor CSI sequence: "\x1b[38;2;255;" (missing g, b, 'm')
        out.extend_from_slice(b"\x1b[38;2;255;");
        // Inject truncated mouse SGR sequence: "\x1b[<32;12" (missing ';' y and 'M')
        out.extend_from_slice(b"\x1b[<32;12");
        // Inject truncated OSC title sequence: "\x1b]0;Unterminated Title" (missing BEL / ST)
        out.extend_from_slice(b"\x1b]0;Unterminated Title");

        out.extend_from_slice(&input[mid..]);
        out
    }

    /// Generates a deterministic sequence of window size pairs (w, h) for testing layout robustness during SIGWINCH storms.
    pub fn fuzz_resize_storm(&self, count: usize, min_size: (u16, u16), max_size: (u16, u16)) -> Vec<(u16, u16)> {
        let mut resizes = Vec::with_capacity(count);
        let mut state: u32 = 0xDEADBEEF;

        let w_span = (max_size.0.saturating_sub(min_size.0) + 1) as u32;
        let h_span = (max_size.1.saturating_sub(min_size.1) + 1) as u32;

        for _ in 0..count {
            // LCG pseudo-random generator
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            let w = min_size.0 + if w_span > 0 { (state % w_span) as u16 } else { 0 };

            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            let h = min_size.1 + if h_span > 0 { (state % h_span) as u16 } else { 0 };

            resizes.push((w, h));
        }

        resizes
    }

    /// Progressively decodes fragmented byte chunks into a valid UTF-8 string,
    /// buffering partial multi-byte sequences until completed.
    pub fn decode_fragmented_utf8(chunks: &[Vec<u8>]) -> (String, usize) {
        let mut decoded = String::new();
        let mut pending_bytes = Vec::new();
        let mut chunks_processed = 0;

        for chunk in chunks {
            chunks_processed += 1;
            pending_bytes.extend_from_slice(chunk);

            // Attempt to consume valid UTF-8 from the front
            let mut valid_up_to = 0;
            while valid_up_to < pending_bytes.len() {
                match std::str::from_utf8(&pending_bytes[valid_up_to..]) {
                    Ok(valid_str) => {
                        decoded.push_str(valid_str);
                        valid_up_to = pending_bytes.len();
                    }
                    Err(e) => {
                        let valid_len = e.valid_up_to();
                        if valid_len > 0 {
                            let valid_slice = &pending_bytes[valid_up_to..valid_up_to + valid_len];
                            decoded.push_str(std::str::from_utf8(valid_slice).unwrap());
                            valid_up_to += valid_len;
                        }

                        if let Some(error_len) = e.error_len() {
                            decoded.push('\u{FFFD}');
                            valid_up_to += error_len;
                        } else {
                            // Incomplete sequence at buffer boundary; leave for next chunk
                            break;
                        }
                    }
                }
            }

            // Retain unconsumed trailing incomplete bytes
            if valid_up_to > 0 {
                pending_bytes.drain(..valid_up_to);
            }
        }

        (decoded, chunks_processed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multibyte_utf8_fragmentation() {
        let fuzzer = ChaosStreamFuzzer::new();
        // 4-byte crab emoji and greek characters: "🦀 — αβγ"
        let input = "🦀 — αβγ".as_bytes();
        let chunks = fuzzer.fuzz_payload(input);

        // Multiple small chunks produced
        assert!(chunks.len() > 1);

        // Progressively reconstructed stream must match original exactly
        let (reconstructed, processed) = ChaosStreamFuzzer::decode_fragmented_utf8(&chunks);
        assert_eq!(reconstructed, "🦀 — αβγ");
        assert_eq!(processed, chunks.len());
    }

    #[test]
    fn test_malformed_ansi_injection() {
        let fuzzer = ChaosStreamFuzzer::new();
        let input = b"Hello, World!";
        let malformed = fuzzer.inject_malformed_ansi(input);

        assert!(malformed.len() > input.len());
        // Contains broken ESC sequences
        assert!(malformed.windows(4).any(|w| w == b"\x1b[38"));
        assert!(malformed.windows(4).any(|w| w == b"\x1b[<3"));
    }

    #[test]
    fn test_resize_storm_generation() {
        let fuzzer = ChaosStreamFuzzer::new();
        let storm = fuzzer.fuzz_resize_storm(1000, (20, 5), (300, 120));
        assert_eq!(storm.len(), 1000);

        for (w, h) in storm {
            assert!((20..=300).contains(&w));
            assert!((5..=120).contains(&h));
        }
    }
}
