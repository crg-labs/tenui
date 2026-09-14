use tenui_core::{buffer::Buffer, canvas::CanvasSubviewMut, geometry::Rect};

use crate::{
    border::{BorderCompositor, FrameConfig},
    filter::BackdropFilter,
    glow::GlowEffect,
    shader::CellShader,
    shadow::{DropShadow, FocusHalo},
};

/// Multi-Pass Surface Rasterization Orchestrator.
pub struct VfxPipeline;

impl VfxPipeline {
    /// Executes Pass A: Optical Backdrop Filter over an intercepted subview slice.
    pub fn pass_a_backdrop(buffer: &mut Buffer, modal_bounds: Rect, filter: &BackdropFilter) {
        let mut slice = buffer.subview_mut(modal_bounds);
        filter.apply(&mut slice);
    }

    /// Executes Pass B: Surface Volumetrics & Border Geometry.
    pub fn pass_b_surface_and_frame(
        surface: &mut CanvasSubviewMut,
        frame: Option<&FrameConfig>,
        inset_shadow_radius: Option<f32>,
    ) {
        if let Some(radius) = inset_shadow_radius {
            crate::surface::SurfaceVolumetrics::render_inset_shadow(surface, radius, 0.45);
        }

        if let Some(cfg) = frame {
            BorderCompositor::render_frame(surface, cfg);
        }
    }

    /// Executes Pass D: Outer Convolutions (Drop Shadow, Focus Halo, and Glow).
    pub fn pass_d_outer_convolutions(
        surface: &mut CanvasSubviewMut,
        content_bounds: Rect,
        shadow: Option<&DropShadow>,
        focus_halo: Option<&FocusHalo>,
        elapsed_secs: f32,
    ) {
        if let Some(s) = shadow {
            s.render_shadow(surface, content_bounds);
        }

        if let Some(halo) = focus_halo {
            halo.render_halo(surface, content_bounds, elapsed_secs);
        }
    }

    /// Executes Pass D+: Glow emission around or within a content region.
    pub fn pass_d_glow(surface: &mut CanvasSubviewMut, content_bounds: Rect, glow: &GlowEffect, elapsed_secs: f32) {
        glow.render(surface, content_bounds, elapsed_secs);
    }

    /// Executes Pass F: Global / Container Screen Shaders.
    pub fn pass_f_screen_shader(surface: &mut CanvasSubviewMut, shader: &dyn CellShader, elapsed_secs: f32) {
        let bounds = surface.bounds;
        for y in 0..bounds.height {
            for x in 0..bounds.width {
                if let Some(cell) = surface.get_cell_mut(x, y) {
                    shader.shade_cell(x, y, bounds, cell, elapsed_secs);
                }
            }
        }
    }
}
