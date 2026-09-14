#![forbid(unsafe_code)]
//! # Tenui WGPU (`tenui-wgpu`)
//!
//! Dual-head native GPU canvas runner and WGSL shader rasterizer.
//!
//! `tenui-wgpu` renders Tenui cell buffers directly onto hardware GPU surfaces
//! (via wgpu / WebGPU / DirectX / Metal / Vulkan):
//!
//! ## Core Primitives
//!
//! - **[`WgpuCanvasRunner`]**: Converts CPU cell buffers into GPU instanced vertex streams
//!   ([`WgpuCanvasRunner::update_from_buffer`]).
//! - **[`CellInstance`]**: Memory-mapped GPU vertex instance storing per-cell grid coordinates,
//!   linear sRGB foreground/background colors, glyph texture atlas UV coordinates, and modifier bitfields.
//! - **[`GpuPipelineConfig`]**: Pipeline parameters including grid dimensions, cell pixel resolution,
//!   and post-process CRT scanline emulation.
//! - **[`TERMINAL_WGSL_SHADER`]**: Embedded WGSL shader source code.
//!
//! ## Runnable Example: Instanced Buffer Generation
//!
//! ```rust
//! use tenui_core::{Buffer, Color, Rect};
//! use tenui_wgpu::{GpuPipelineConfig, WgpuCanvasRunner};
//!
//! let config = GpuPipelineConfig {
//!     grid_cols: 10,
//!     grid_rows: 5,
//!     enable_crt_scanlines: false,
//!     cell_pixel_size: (10, 20),
//! };
//!
//! let mut runner = WgpuCanvasRunner::new(config);
//! let buffer = Buffer::empty(Rect::new(0, 0, 10, 5));
//!
//! // Convert buffer cells into GPU instances
//! runner.update_from_buffer(&buffer);
//! assert_eq!(runner.instance_buffer.len(), 50);
//! ```

pub mod pipeline;

pub use pipeline::{CellInstance, GpuPipelineConfig, TERMINAL_WGSL_SHADER, WgpuCanvasRunner};
