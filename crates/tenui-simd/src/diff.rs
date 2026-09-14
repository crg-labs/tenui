#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

use tenui_core::buffer::Buffer;

#[repr(C, align(32))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AlignedCell {
    pub grapheme: u64,
    pub fg: u64,
    pub bg: u64,
    pub modifier: u64,
}

impl AlignedCell {
    pub const ZERO: Self = Self {
        grapheme: 0,
        fg: 0,
        bg: 0,
        modifier: 0,
    };

    pub fn new(grapheme: u64, fg: u64, bg: u64, modifier: u64) -> Self {
        Self {
            grapheme,
            fg,
            bg,
            modifier,
        }
    }

    /// Packs a `Cell` into 256 bits **losslessly for diffing**: two cells produce equal
    /// `AlignedCell`s iff the cells are equal. Every diff-relevant field is encoded — the
    /// grapheme (inline bytes, or the interned pool id for spillover EGCs), both colors by
    /// their variant-preserving `diff_key`, and modifier + underline style/color + width +
    /// continuation flag folded into the modifier lane — so the SIMD movemask never reports
    /// a false "unchanged" (which would drop a real screen update).
    pub fn from_cell(cell: &tenui_core::Cell) -> Self {
        let sym = cell.symbol.as_str();
        let grapheme = if cell.symbol.is_spillover() {
            // Spillover EGC (>7 bytes): the interned pool id is unique per distinct
            // grapheme and dedup'd, so it identifies the symbol without truncation.
            (1u64 << 63) | (tenui_core::cell::intern_egc(sym) as u64)
        } else {
            let mut g_bytes = [0u8; 8];
            let s = sym.as_bytes();
            let n = s.len().min(7); // inline symbols are <=7 bytes; bit 63 stays free
            g_bytes[..n].copy_from_slice(&s[..n]);
            u64::from_le_bytes(g_bytes)
        };

        let fg = cell.fg.diff_key() as u64;
        let bg = cell.bg.diff_key() as u64;

        // modifier lane: modifier(16) | ul_style(3) | is_continuation(1) | width(8) | ul_color_key(32)
        let modifier = (cell.modifier.bits() as u64)
            | ((cell.underline_style as u64) << 16)
            | ((cell.is_continuation as u64) << 19)
            | ((cell.width as u64) << 20)
            | ((cell.underline_color.diff_key() as u64) << 28);

        Self {
            grapheme,
            fg,
            bg,
            modifier,
        }
    }
}

pub struct SimdDiffScanner;

impl SimdDiffScanner {
    /// Differences two `Buffer`s (front vs back), returning the flat indices of every
    /// changed cell — the accelerated replacement for a scalar per-cell scan, targeting
    /// large (4K/8K) viewports. Packs cells into 256-bit lanes via the
    /// lossless [`AlignedCell::from_cell`], so the result exactly matches a full `Cell`
    /// comparison. `Vec<AlignedCell>` is 32-byte aligned, satisfying the aligned SIMD load.
    pub fn diff_buffers(front: &Buffer, back: &Buffer) -> Vec<usize> {
        // Reuse per-thread lane scratch across frames so the hot path allocates nothing
        // after warm-up (a fresh Vec per call is ~2·N·32 bytes — measurable at 60 fps).
        thread_local! {
            static SCRATCH: std::cell::RefCell<(Vec<AlignedCell>, Vec<AlignedCell>)> =
                const { std::cell::RefCell::new((Vec::new(), Vec::new())) };
        }
        SCRATCH.with(|cell| {
            let (fa, ba) = &mut *cell.borrow_mut();
            fa.clear();
            ba.clear();
            let n = front.cells.len().min(back.cells.len());
            fa.reserve(n);
            ba.reserve(n);
            for i in 0..n {
                fa.push(AlignedCell::from_cell(&front.cells[i]));
                ba.push(AlignedCell::from_cell(&back.cells[i]));
            }
            let mut dirty = Vec::new();
            Self::scan_dirty_runs(ba, fa, &mut dirty);
            dirty
        })
    }

    /// Compares two cell slices, returning indices of modified cells. Dispatches to the
    /// best available vector backend (AVX-512 → AVX2 → NEON) with a portable fallback.
    pub fn scan_dirty_runs(back: &[AlignedCell], front: &[AlignedCell], out_dirty_indices: &mut Vec<usize>) {
        #[cfg(all(target_arch = "x86_64", feature = "avx512"))]
        {
            if is_x86_feature_detected!("avx512f") {
                unsafe {
                    return Self::scan_dirty_runs_avx512(back, front, out_dirty_indices);
                }
            }
        }
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                unsafe {
                    return Self::scan_dirty_runs_avx2(back, front, out_dirty_indices);
                }
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            if std::arch::is_aarch64_feature_detected!("neon") {
                unsafe {
                    return Self::scan_dirty_runs_neon(back, front, out_dirty_indices);
                }
            }
        }

        Self::scan_dirty_runs_fallback(back, front, out_dirty_indices);
    }

    /// # Safety
    /// The CPU must support AVX2 (guaranteed when reached via [`Self::scan_dirty_runs`], which
    /// checks `is_x86_feature_detected!("avx2")`). `AlignedCell` is 32-byte aligned so the
    /// aligned 256-bit loads are valid.
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    pub unsafe fn scan_dirty_runs_avx2(
        back: &[AlignedCell],
        front: &[AlignedCell],
        out_dirty_indices: &mut Vec<usize>,
    ) {
        let len = back.len().min(front.len());
        let mut i = 0;

        // Process 2 cells (64 bytes) per iteration using 256-bit SIMD registers
        while i + 2 <= len {
            // SAFETY: caller guarantees AVX2 support; AlignedCell is 32-byte aligned
            let (mask0, mask1) = unsafe {
                let b0 = _mm256_load_si256(back.as_ptr().add(i) as *const __m256i);
                let f0 = _mm256_load_si256(front.as_ptr().add(i) as *const __m256i);
                let cmp0 = _mm256_cmpeq_epi64(b0, f0);

                let b1 = _mm256_load_si256(back.as_ptr().add(i + 1) as *const __m256i);
                let f1 = _mm256_load_si256(front.as_ptr().add(i + 1) as *const __m256i);
                let cmp1 = _mm256_cmpeq_epi64(b1, f1);

                (_mm256_movemask_epi8(cmp0), _mm256_movemask_epi8(cmp1))
            };

            // -1 (0xFFFFFFFF) means all 32 bytes match
            if mask0 != -1 {
                out_dirty_indices.push(i);
            }
            if mask1 != -1 {
                out_dirty_indices.push(i + 1);
            }

            i += 2;
        }

        // Handle trailing scalar cells
        while i < len {
            if back[i] != front[i] {
                out_dirty_indices.push(i);
            }
            i += 1;
        }
    }

    /// AVX-512 backend: 8 × 64-bit lanes = two 32-byte cells per 512-bit compare.
    /// Feature-gated (`avx512` cargo feature) because the intrinsics require a recent
    /// toolchain; the default build uses AVX2 + fallback.
    ///
    /// # Safety
    /// The CPU must support AVX-512F (checked by [`scan_dirty_runs`] before dispatch).
    #[cfg(all(target_arch = "x86_64", feature = "avx512"))]
    #[target_feature(enable = "avx512f")]
    pub unsafe fn scan_dirty_runs_avx512(
        back: &[AlignedCell],
        front: &[AlignedCell],
        out_dirty_indices: &mut Vec<usize>,
    ) {
        let len = back.len().min(front.len());
        let mut i = 0;
        while i + 2 <= len {
            // SAFETY: caller guarantees AVX-512F support; pointers are in-bounds
            let mask = unsafe {
                let b = _mm512_loadu_si512(back.as_ptr().add(i) as *const i32);
                let f = _mm512_loadu_si512(front.as_ptr().add(i) as *const i32);
                _mm512_cmpeq_epi64_mask(b, f)
            };
            if (mask & 0x0F) != 0x0F {
                out_dirty_indices.push(i);
            }
            if (mask & 0xF0) != 0xF0 {
                out_dirty_indices.push(i + 1);
            }
            i += 2;
        }
        while i < len {
            if back[i] != front[i] {
                out_dirty_indices.push(i);
            }
            i += 1;
        }
    }

    /// ARM NEON backend: each 32-byte cell is two 128-bit (2 × u64) lanes; a cell is
    /// unchanged only if both halves compare all-equal.
    ///
    /// # Safety
    /// The CPU must support NEON (checked by [`scan_dirty_runs`] before dispatch); always
    /// available on aarch64 in practice.
    #[cfg(target_arch = "aarch64")]
    #[target_feature(enable = "neon")]
    pub unsafe fn scan_dirty_runs_neon(
        back: &[AlignedCell],
        front: &[AlignedCell],
        out_dirty_indices: &mut Vec<usize>,
    ) {
        use std::arch::aarch64::*;
        let len = back.len().min(front.len());
        for i in 0..len {
            // SAFETY: caller guarantees NEON support; pointers are in-bounds
            let min = unsafe {
                let bp = back.as_ptr().add(i) as *const u64;
                let fp = front.as_ptr().add(i) as *const u64;
                let b_lo = vld1q_u64(bp);
                let f_lo = vld1q_u64(fp);
                let b_hi = vld1q_u64(bp.add(2));
                let f_hi = vld1q_u64(fp.add(2));
                let eq_lo = vceqq_u64(b_lo, f_lo);
                let eq_hi = vceqq_u64(b_hi, f_hi);
                let both_eq = vandq_u64(eq_lo, eq_hi);
                vgetq_lane_u64(both_eq, 0).min(vgetq_lane_u64(both_eq, 1))
            };
            if min != u64::MAX {
                out_dirty_indices.push(i);
            }
        }
    }

    pub fn scan_dirty_runs_fallback(back: &[AlignedCell], front: &[AlignedCell], out_dirty_indices: &mut Vec<usize>) {
        let len = back.len().min(front.len());
        let mut i = 0;

        // Process 4 cells per unrolled step
        while i + 4 <= len {
            if back[i] != front[i] {
                out_dirty_indices.push(i);
            }
            if back[i + 1] != front[i + 1] {
                out_dirty_indices.push(i + 1);
            }
            if back[i + 2] != front[i + 2] {
                out_dirty_indices.push(i + 2);
            }
            if back[i + 3] != front[i + 3] {
                out_dirty_indices.push(i + 3);
            }
            i += 4;
        }

        while i < len {
            if back[i] != front[i] {
                out_dirty_indices.push(i);
            }
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_diff_scanner_exact_indices() {
        let back = vec![AlignedCell::ZERO; 16];
        let mut front = vec![AlignedCell::ZERO; 16];

        // Modify cells at index 3, 7, 15
        front[3].grapheme = 42;
        front[7].fg = 0x00FF00;
        front[15].modifier = 1;

        let mut dirty = Vec::new();
        SimdDiffScanner::scan_dirty_runs(&back, &front, &mut dirty);

        assert_eq!(dirty, vec![3, 7, 15]);

        // When identical, out_dirty_indices is empty
        dirty.clear();
        SimdDiffScanner::scan_dirty_runs(&front, &front, &mut dirty);
        assert!(dirty.is_empty());
    }

    #[test]
    fn test_aligned_cell_layout_32_bytes() {
        assert_eq!(std::mem::size_of::<AlignedCell>(), 32);
        assert_eq!(std::mem::align_of::<AlignedCell>(), 32);
    }

    /// Lossless invariant: two cells encode equal iff the cells are equal — including the
    /// cases `to_rgb` would have collapsed (Reset vs same-rgb, Indexed vs Rgb, underline).
    #[test]
    fn test_aligned_cell_is_lossless_for_diffing() {
        use tenui_core::{Cell, Color, Modifier, UnderlineStyle};

        let pairs_differ = [
            (Cell::new("a"), Cell::new("b")),
            (Cell::new("x"), {
                let mut c = Cell::new("x");
                c.fg = Color::Rgb(0, 0, 0);
                c // Reset-default fg vs explicit black-rgb: different variants
            }),
            (
                {
                    let mut c = Cell::new("x");
                    c.fg = Color::Indexed(1);
                    c
                },
                {
                    let mut c = Cell::new("x");
                    c.fg = Color::Rgb(0, 0, 1);
                    c
                },
            ),
            (Cell::new("x"), {
                let mut c = Cell::new("x");
                c.underline_style = UnderlineStyle::Curly;
                c
            }),
            (Cell::new("x"), {
                let mut c = Cell::new("x");
                c.underline_color = Color::Rgb(9, 9, 9);
                c
            }),
            (Cell::new("x"), {
                let mut c = Cell::new("x");
                c.modifier = Modifier::BOLD;
                c
            }),
        ];
        for (a, b) in &pairs_differ {
            assert_ne!(a, b);
            assert_ne!(
                AlignedCell::from_cell(a),
                AlignedCell::from_cell(b),
                "cells differ but encoded equal: {:?} vs {:?}",
                a,
                b
            );
        }

        // Equal cells encode equal.
        let c = Cell::new("z");
        assert_eq!(AlignedCell::from_cell(&c), AlignedCell::from_cell(&c.clone()));
    }

    /// `diff_buffers` must return exactly the scalar `Cell`-comparison dirty set.
    #[test]
    fn test_diff_buffers_matches_scalar_reference() {
        use tenui_core::{Buffer, Cell, Color, Modifier};

        let w = 37u16;
        let h = 11u16;
        let front = Buffer::new(w, h);
        let mut back = Buffer::new(w, h);

        // Deterministic pseudo-random edits on `back`.
        let mut seed = 0x9E3779B9u32;
        let mut next = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            seed
        };
        for _ in 0..80 {
            let idx = (next() as usize) % (w as usize * h as usize);
            let x = (idx % w as usize) as u16;
            let y = (idx / w as usize) as u16;
            if let Some(cell) = back.get_mut(x, y) {
                let mut c = Cell::from_char((b'a' + (next() % 26) as u8) as char);
                c.fg = Color::Indexed((next() % 256) as u8);
                if next() % 2 == 0 {
                    c.modifier = Modifier::BOLD;
                }
                *cell = c;
            }
        }

        // Scalar reference dirty set.
        let mut scalar: Vec<usize> = Vec::new();
        for i in 0..front.cells.len() {
            if front.cells[i] != back.cells[i] {
                scalar.push(i);
            }
        }

        let mut simd = SimdDiffScanner::diff_buffers(&front, &back);
        simd.sort_unstable();
        assert_eq!(simd, scalar);
    }
}
