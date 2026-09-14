#![forbid(unsafe_code)]
//! # Tenui VFX (`tenui-vfx`)
//!
//! Multi-pass spatial compositing, optical filtering, tactile surfaces, and procedural cell shaders.
//!
//! `tenui-vfx` provides graphical embellishments for modern terminal applications, enabling
//! visual depth, focus indicators, and dynamic feedback through multiple rasterization passes:
//!
//! ## Subsystems
//!
//! - **[`VfxPipeline`]**: Multi-pass orchestrator coordinating optical backdrop filters,
//!   surface volumetrics, outer convolutions, and procedural cell shaders.
//! - **Optical Filters ([`AcrylicFilter`], [`BackdropFilter`])**: Frosted glass effects that sample,
//!   dim, blur, and tint background cells behind modal dialogs and floating menus.
//! - **Procedural Shaders ([`CellShader`], [`CrtScanlineShader`], [`SinusoidalShimmerShader`])**:
//!   Screen-space mathematical pixel shaders operating over terminal cells.
//! - **Shadows & Halos ([`DropShadow`], [`FocusHalo`])**: Drop shadow casting with directional offset
//!   and pulsating focus halos indicating active keyboard navigation targets.
//! - **Particle System ([`ParticlePool`], [`ParticleEmitter`])**: Lightweight 2D kinematic particle
//!   simulation for bursts, fire, sparks, and celebrate effects.
//! - **Tactile Surfaces ([`TactileButton`], [`SurfaceVolumetrics`])**: Inset shadows and pressed-state
//!   depth visualizers simulating physical buttons.
//! - **Text Optics ([`TextOpticsCompositor`], [`TextEffect`])**: Wave, shimmer, and chromatic
//!   aberration styling over string runs.

pub mod border;
pub mod button;
pub mod filter;
pub mod glow;
pub mod particles;
pub mod pipeline;
pub mod region;
pub mod shader;
pub mod shadow;
pub mod spinner;
pub mod surface;
pub mod text;

pub mod prelude {
    pub use crate::{
        border::{BorderCompositor, BorderCorner, FrameConfig},
        button::{ButtonVariant, SplitActionButton, TactileButton},
        filter::{AcrylicFilter, BackdropFilter},
        glow::{BloomStack, GlowEffect, GlowFalloff},
        particles::{Particle, ParticleEmitter, ParticlePool},
        pipeline::VfxPipeline,
        region::{RegionStencilMask, SceneTransition, WarpField},
        shader::{CellShader, CrtScanlineShader, SinusoidalShimmerShader},
        shadow::{DropShadow, FocusHalo},
        spinner::VectorArcSpinner,
        surface::SurfaceVolumetrics,
        text::{TextEffect, TextOpticsCompositor},
    };
}

pub use prelude::*;
