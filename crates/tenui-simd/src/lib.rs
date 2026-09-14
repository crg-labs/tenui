//! # Tenui SIMD (`tenui-simd`)
//!
//! SIMD-vectorized differential buffer scanning and dirty run detection.
//!
//! `tenui-simd` accelerates terminal frame rendering for high-resolution displays
//! (e.g. 4K, 8K terminals) by comparing front and back buffers using vector instructions.
//!
//! ## Architecture
//!
//! - **[`AlignedCell`]**: 32-byte 256-bit aligned representation of a terminal cell. Losslessly packs
//!   the grapheme symbol (or interned EGC index), foreground color key, background color key,
//!   and modifier bits. Two cells produce identical `AlignedCell`s if and only if the cells are visually identical.
//! - **[`SimdDiffScanner`]**: Scans cell slices and returns flat indices of modified cells.
//!   Dynamically dispatches to the optimal vector instruction set at runtime:
//!   - **AVX-512**: Compares eight 32-byte cells per vector iteration.
//!   - **AVX2**: Compares four 32-byte cells per vector iteration using 256-bit vector comparisons and movemasks.
//!   - **NEON**: ARM 128-bit vector comparisons.
//!   - **Portable Scalar Fallback**: Standard word-at-a-time comparison on platforms without vector extensions.
//!
//! ## Runnable Example
//!
//! ```rust
//! use tenui_core::{Buffer, Color, Modifier, Rect};
//! use tenui_simd::SimdDiffScanner;
//!
//! let rect = Rect::new(0, 0, 10, 2);
//! let front = Buffer::empty(rect);
//! let mut back = Buffer::empty(rect);
//!
//! // Mutate one cell in the back buffer
//! back.set_char(5, 0, 'X', Color::Cyan, Color::Reset, Modifier::BOLD);
//!
//! // Perform SIMD differential scanning
//! let dirty_indices = SimdDiffScanner::diff_buffers(&front, &back);
//!
//! assert_eq!(dirty_indices.len(), 1);
//! assert_eq!(dirty_indices[0], 5);
//! ```

pub mod diff;

pub use diff::{AlignedCell, SimdDiffScanner};
