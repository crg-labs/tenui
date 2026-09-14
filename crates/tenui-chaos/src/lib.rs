#![forbid(unsafe_code)]
//! # Tenui Chaos (`tenui-chaos`)
//!
//! Deterministic chaos engineering and PTY stream fuzzing suite.
//!
//! `tenui-chaos` tests terminal parsers and event loops against real-world network disruptions,
//! packet fragmentation, and terminal emulator corruption:
//!
//! ## Core Primitives
//!
//! - **[`ChaosStreamFuzzer`]**: Splits clean terminal byte streams into fragmented micro-chunks
//!   (breaking multi-byte UTF-8 sequences and ANSI escape sequences across buffer boundaries),
//!   simulates byte drops, and injects truncated CSI/OSC control sequences.
//!
//! ## Runnable Example: Micro-Chunk Fragmentation
//!
//! ```rust
//! use tenui_chaos::ChaosStreamFuzzer;
//!
//! let fuzzer = ChaosStreamFuzzer::new().with_fragment_multibyte(true);
//! let payload = "🔥 Tenui TUI 🔥".as_bytes();
//!
//! // Fragments stream into small byte packets across multi-byte boundaries
//! let fragments = fuzzer.fuzz_payload(payload);
//! assert!(fragments.len() > 1);
//!
//! // Recombining fragments yields the original valid string
//! let reconstructed: Vec<u8> = fragments.into_iter().flatten().collect();
//! assert_eq!(reconstructed, payload);
//! ```

pub mod fuzzer;

pub use fuzzer::ChaosStreamFuzzer;
