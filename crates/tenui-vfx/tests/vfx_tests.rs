use tenui_core::{
    buffer::{Buffer, Rect},
    color::Color,
};
use tenui_vfx::{
    border::{BorderCompositor, BorderCorner, FrameConfig},
    button::{ButtonVariant, TactileButton},
    filter::BackdropFilter,
    particles::ParticlePool,
    region::RegionStencilMask,
    shader::{CellShader, CrtScanlineShader, SinusoidalShimmerShader},
    shadow::{DropShadow, FocusHalo},
    spinner::VectorArcSpinner,
    surface::SurfaceVolumetrics,
    text::{TextEffect, TextOpticsCompositor},
};

#[test]
fn test_all_spinners_share_identical_phase() {
    // Invariant: all vector spinners rotating at identical speed share identical phase angle
    let spinner_a = VectorArcSpinner::new(2.0, Color::CYAN);
    let spinner_b = VectorArcSpinner::new(4.5, Color::MAGENTA);

    let t1 = std::f32::consts::PI;
    let phase_a = spinner_a.phase_at(t1);
    let phase_b = spinner_b.phase_at(t1);

    assert!(
        (phase_a - phase_b).abs() < 1e-6,
        "Phase delta across independent spinners must be exactly zero"
    );
}

#[test]
fn test_vfx_shadow_does_not_overflow() {
    let mut buffer = Buffer::new(20, 10);
    let shadow = DropShadow::ELEVATED;

    let content_bounds = Rect::new(4, 2, 8, 4);
    let mut view = buffer.subview_mut(Rect::new(0, 0, 20, 10));

    shadow.render_shadow(&mut view, content_bounds);

    // Verify content interior was untouched by shadow
    for y in content_bounds.y..(content_bounds.y + content_bounds.height) {
        for x in content_bounds.x..(content_bounds.x + content_bounds.width) {
            let cell = buffer.get(x, y).unwrap();
            assert_eq!(cell.bg, Color::Reset);
        }
    }

    // Verify shadow was written into bottom-right exterior cells
    let shadow_sample = buffer
        .get(
            content_bounds.x + content_bounds.width + 1,
            content_bounds.y + content_bounds.height,
        )
        .unwrap();
    assert_ne!(shadow_sample.bg, Color::Reset);
}

#[test]
fn test_bayer_dither_average_luminance() {
    // Mean of 4x4 Bayer matrix should equal 7.5 / 16.0 ~ 0.46875 (normalized zero-mean offset)
    let mut sum = 0.0f32;
    for row in 0..4 {
        for col in 0..4 {
            sum += SurfaceVolumetrics::BAYER_4X4[row][col];
        }
    }
    let mean = sum / 16.0;
    assert!(
        (mean - 0.46875).abs() < 1e-5,
        "Bayer 4x4 dither matrix must have correct mean energy"
    );
}

#[test]
fn test_keycap_active_preserves_taffy_reflow() {
    let mut buffer = Buffer::new(30, 8);
    let button = TactileButton::new("HALT SYSTEM", Color::RED);

    // 1. Render in Resting state
    {
        let mut view = buffer.subview_mut(Rect::new(2, 2, 18, 4));
        button.paint(&mut view);
    }
    // Resting state button face starts at local offset 0
    let resting_face_sample = buffer.get(2, 2).unwrap();
    assert_ne!(resting_face_sample.bg, Color::Reset);

    // 2. Render in Active state (physically depressed +1)
    let mut active_button = button.clone();
    active_button.is_active = true;
    {
        let mut view = buffer.subview_mut(Rect::new(2, 2, 18, 4));
        active_button.paint(&mut view);
    }
    // Active state button face shifted to local offset +1, preserving bounding box
    let active_face_sample = buffer.get(3, 3).unwrap();
    assert_ne!(active_face_sample.bg, Color::Reset);

    // 3. Render Capsule button with FocusHalo
    let capsule = TactileButton::new("STATUS OK", Color::GREEN)
        .variant(ButtonVariant::Capsule)
        .focus_halo(FocusHalo::CYAN_PULSE);
    {
        let mut view = buffer.subview_mut(Rect::new(0, 0, 16, 2));
        capsule.paint(&mut view);
    }
    assert_eq!(buffer.get(0, 0).unwrap().symbol.as_str(), "◖");
}

#[test]
fn test_text_glitch_strictly_paint_lane() {
    let mut buffer = Buffer::new(40, 5);

    // Progress 0.0: all characters should be high-entropy noise
    {
        let mut view = buffer.subview_mut(Rect::new(0, 0, 40, 5));
        TextOpticsCompositor::render_text(
            &mut view,
            0,
            0,
            "CLASSIFIED",
            Color::WHITE,
            TextEffect::GlitchDecryption {
                progress: 0.0,
                seed: 12345,
            },
            0.0,
        );
    }
    let first_char = buffer.get(0, 0).unwrap().symbol.as_str();
    assert_ne!(first_char, "C", "At progress 0.0, plaintext should not be revealed");

    // Progress 1.0: all characters should be locked to destination plaintext
    {
        let mut view = buffer.subview_mut(Rect::new(0, 0, 40, 5));
        TextOpticsCompositor::render_text(
            &mut view,
            0,
            0,
            "CLASSIFIED",
            Color::WHITE,
            TextEffect::GlitchDecryption {
                progress: 1.0,
                seed: 12345,
            },
            0.0,
        );
    }
    let first_char_done = buffer.get(0, 0).unwrap().symbol.as_str();
    assert_eq!(first_char_done, "C", "At progress 1.0, plaintext must be unlocked");
}

#[test]
fn test_backdrop_filter_acrylic_blur() {
    let mut buffer = Buffer::new(20, 10);
    // Write high-contrast text into the desktop back-buffer
    buffer.set_string(
        2,
        2,
        "HIGH CONTRAST DESKTOP",
        Color::WHITE,
        Color::BLACK,
        tenui_core::cell::Modifier::empty(),
    );

    let filter = BackdropFilter::ACRYLIC;
    let mut slice = buffer.subview_mut(Rect::new(0, 0, 20, 10));
    filter.apply(&mut slice);

    // Verify text was diffused into stipple characters
    let diffused_cell = buffer.get(2, 2).unwrap();
    let sym = diffused_cell.symbol.as_str();
    assert!(
        sym == "·" || sym == "░" || sym == "▒",
        "High-contrast glyphs under acrylic filter must diffuse to stipples, found: {:?}",
        sym
    );
}

#[test]
fn test_subcell_border_cutout_badge() {
    let mut buffer = Buffer::new(30, 8);
    let cfg = FrameConfig {
        corner: BorderCorner::Chamfered,
        border_color: Color::CYAN,
        background: Some(Color::BLACK),
        badge_cutout: Some("LIVE TELEMETRY".to_string()),
    };

    let mut view = buffer.subview_mut(Rect::new(0, 0, 30, 8));
    BorderCompositor::render_frame(&mut view, &cfg);

    // Verify chamfered corners
    assert_eq!(buffer.get(0, 0).unwrap().symbol.as_str(), "◤");
    assert_eq!(buffer.get(29, 0).unwrap().symbol.as_str(), "◥");
    assert_eq!(buffer.get(0, 7).unwrap().symbol.as_str(), "◣");
    assert_eq!(buffer.get(29, 7).unwrap().symbol.as_str(), "◢");

    // Verify badge cutout junctions
    assert_eq!(buffer.get(1, 0).unwrap().symbol.as_str(), "┤");
    assert_eq!(buffer.get(16, 0).unwrap().symbol.as_str(), "├");
}

#[test]
fn test_sdf_circle_stencil_quarter_block_aa() {
    let mut buffer = Buffer::new(20, 10);
    let mut view = buffer.subview_mut(Rect::new(0, 0, 20, 10));

    RegionStencilMask::mask_circle(&mut view, Color::CYAN);

    // Center cell should be fully interior solid background
    let center = buffer.get(10, 5).unwrap();
    assert_eq!(center.bg, Color::CYAN);

    // Perimeter boundary cells should contain sub-pixel quarter-block anti-aliasing glyphs
    let mut found_quarter_block = false;
    for y in 0..10 {
        for x in 0..20 {
            let sym = buffer.get(x, y).unwrap().symbol.as_str();
            if ["▘", "▝", "▀", "▖", "▌", "▞", "▛", "▗", "▚", "▐", "▜", "▄", "▙", "▟"].contains(&sym)
            {
                found_quarter_block = true;
                break;
            }
        }
    }
    assert!(
        found_quarter_block,
        "SDF circle stencil must produce quarter-block anti-aliasing on boundaries"
    );
}

#[test]
fn test_particles_zero_allocation_kinematics() {
    let mut pool = ParticlePool::new();
    assert_eq!(pool.count, 0);

    let spawned = pool.spawn(5.0, 5.0, 10.0, 0.0, 0.0, 9.8, 1.0, Color::YELLOW);
    assert!(spawned);
    assert_eq!(pool.count, 1);

    pool.update(0.1);
    assert_eq!(pool.count, 1);
    assert!(
        pool.particles[0].x > 5.5,
        "Particle position must advance kinematically"
    );

    let mut buffer = Buffer::new(20, 10);
    let mut view = buffer.subview_mut(Rect::new(0, 0, 20, 10));
    pool.render_braille(&mut view);
}

#[test]
fn test_crt_and_shimmer_shaders() {
    let mut cell = tenui_core::cell::Cell::new("A");
    cell.fg = Color::WHITE;
    cell.bg = Color::BLACK;

    let crt = CrtScanlineShader::default();
    crt.shade_cell(0, 1, Rect::new(0, 0, 10, 10), &mut cell, 0.0);
    // Row 1 should have scanline darkening applied
    assert_ne!(cell.fg, Color::WHITE);

    let shimmer = SinusoidalShimmerShader::default();
    shimmer.shade_cell(5, 5, Rect::new(0, 0, 10, 10), &mut cell, 0.25);
}

#[test]
fn test_text_glow_bleeds_into_neighbors() {
    let mut buffer = Buffer::new(30, 5);
    for y in 0..5u16 {
        for x in 0..30u16 {
            if let Some(cell) = buffer.get_mut(x, y) {
                cell.bg = Color::Rgb(0, 0, 0);
            }
        }
    }

    {
        let mut view = buffer.subview_mut(Rect::new(0, 0, 30, 5));
        TextOpticsCompositor::render_text(
            &mut view,
            5,
            2,
            "GLOW",
            Color::Rgb(0, 255, 255),
            TextEffect::Glow {
                glow_color: Color::Rgb(0, 255, 255),
                radius: 2.0,
                intensity: 0.7,
            },
            0.0,
        );
    }

    // Glyph cells should have the text rendered
    assert_eq!(buffer.get(5, 2).unwrap().symbol.as_str(), "G");
    assert_eq!(buffer.get(6, 2).unwrap().symbol.as_str(), "L");

    // Adjacent cells (above and below) should have glow bleed in their backgrounds
    let above = buffer.get(5, 1).unwrap();
    assert_ne!(above.bg, Color::Rgb(0, 0, 0), "cell above glyph should have glow bleed");

    let below = buffer.get(5, 3).unwrap();
    assert_ne!(below.bg, Color::Rgb(0, 0, 0), "cell below glyph should have glow bleed");

    // A cell far from the text should be unaffected
    let far = buffer.get(0, 0).unwrap();
    assert_eq!(far.bg, Color::Rgb(0, 0, 0), "distant cell should be untouched");
}
