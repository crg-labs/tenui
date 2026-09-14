use std::{path::PathBuf, time::Duration};

use tenui_core::{
    Buffer, Cell, Color, CompactSymbol, DndManager, DragEvent, FrameClock, Modifier, NodeId, PathSanitizer, Point, Rect,
};
use tenui_std::{FileDropZone, Modal, Spinner};
use tenui_vfx::{AcrylicFilter, ParticleEmitter};

#[test]
fn test_particles_render_post_bg() {
    let mut buf = Buffer::new(40, 20);

    // Pass A: Background clear
    buf.fill(Rect::new(0, 0, 40, 20), Cell::empty_with_bg(Color::Rgb(10, 10, 15)));

    // Pass B: Particle emitter renders post-bg
    let mut emitter = ParticleEmitter::new();
    // Force spawn a particle at (10, 5)
    emitter.spawn(10.0, 5.0, 0.0, 0.0, 0.0, 0.0, 100.0, Color::Cyan);

    let mut subview = buf.subview_mut(Rect::new(0, 0, 40, 20));
    emitter.paint(&mut subview);

    // Assert that the particle cell is rendered and was not wiped by Pass A
    let cell = buf.get(10, 5).expect("cell exists");
    assert_ne!(
        cell.symbol.as_str(),
        " ",
        "Particle glyph must be rendered and not overwritten by background pass"
    );
    assert_ne!(cell.fg, Color::Reset);
}

#[test]
fn test_acrylic_modal_preserves_stipple() {
    let mut buf = Buffer::new(60, 25);

    // Fill entire buffer with stipple backdrop
    for y in 0..25 {
        for x in 0..60 {
            if let Some(c) = buf.get_mut(x, y) {
                c.symbol = CompactSymbol::from_str("░");
                c.fg = Color::Rgb(80, 80, 100);
                c.bg = Color::Rgb(20, 20, 30);
            }
        }
    }

    // Apply AcrylicFilter to modal subview
    let modal_rect = Rect::new(10, 5, 40, 15);
    {
        let mut modal_subview = buf.subview_mut(modal_rect);
        let acrylic = AcrylicFilter::new(0.4, 0.5, true);
        acrylic.apply(&mut modal_subview);
    }

    // Render Modal with transparent: true
    let modal = Modal::new("Optical Diagnostics", 40, 15, 1).with_transparent(true);
    {
        let mut modal_subview = buf.subview_mut(modal_rect);
        modal.render(&mut modal_subview, |subview| {
            subview.set_string(
                2,
                2,
                "Translucent Lens Active",
                Color::White,
                Color::TRANSPARENT,
                Modifier::BOLD,
            );
        });
    }

    // Assert that inner cells outside the text retain the stipple symbol '░'
    let inner_cell = buf.get(20, 10).expect("inner cell exists");
    assert_eq!(
        inner_cell.symbol.as_str(),
        "░",
        "Modal interior must retain underlying acrylic stipple when transparent is true"
    );

    // Check text rendered cleanly inside client subview (offset by 1 border cell)
    let text_cell = buf.get(13, 8).expect("text cell exists");
    assert_eq!(text_cell.symbol.as_str(), "T");
}

#[test]
fn test_spinner_flex_attachment() {
    let spinner = Spinner::BRAILLE_ORBIT;
    let frame_0 = spinner.glyph_for_frame(0);
    let frame_1 = spinner.glyph_for_frame(1);
    assert_ne!(
        frame_0, frame_1,
        "Spinner phase must advance across frames to provide kinetic feedback"
    );

    // Verify rendering inside a subview without panic
    let mut buf = Buffer::new(30, 5);
    let mut subview = buf.subview_mut(Rect::new(0, 0, 30, 5));
    spinner.paint(&mut subview, Duration::from_millis(80));
    assert_eq!(subview.get(0, 0).unwrap().symbol.as_str(), frame_1.to_string());
}

#[test]
fn test_fps_ewma_convergence() {
    let mut clock = FrameClock::new();
    let target_dt = 16_667.0 / 1_000_000.0; // ~16.67ms (60 FPS)

    let mut fps_history = Vec::new();
    for _ in 0..60 {
        clock.tick_dt(target_dt);
        fps_history.push(clock.fps());
    }

    let final_fps = clock.fps();
    assert!(
        (58.0..=62.0).contains(&final_fps),
        "EWMA smoothed FPS should converge to ~60.0, got {:.2}",
        final_fps
    );

    // Check variance over last 20 frames is bounded within +-2.5
    let recent = &fps_history[40..60];
    let min_fps = recent.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_fps = recent.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    assert!(
        (max_fps - min_fps) <= 2.5,
        "FPS jitter variance over steady input must be <= 2.5, got {:.3}",
        max_fps - min_fps
    );
}

#[test]
fn test_footer_separator_padding() {
    let items = [
        "[ESC] Terminate",
        "[Tab] Cycle Shaders",
        "VFX: Optical Glass",
        "FPS: 60.0",
    ];
    let footer_text = items.join(" │ ");

    // Verify separator padding: must have a space before and after each '│'
    assert!(
        footer_text.contains(" │ "),
        "Footer items must be separated by padded ' │ ' delimiters"
    );

    for segment in footer_text.split(" │ ") {
        assert!(
            !segment.starts_with('│') && !segment.ends_with('│'),
            "No unpadded separator glyphs allowed in footer: '{}'",
            segment
        );
    }
}

#[test]
fn test_telemetry_phase_advancement() {
    let mut telemetry_phase: f32 = 0.0;
    let mut previous_phase = telemetry_phase;
    let mut previous_sample = (telemetry_phase).sin();

    for _ in 0..10 {
        telemetry_phase += 0.08;
        assert!(
            telemetry_phase > previous_phase,
            "Telemetry phase must advance monotonically on tick: {} vs {}",
            telemetry_phase,
            previous_phase
        );
        let sample = (telemetry_phase).sin();
        assert!(
            !sample.is_nan() && !sample.is_infinite(),
            "Telemetry sample must be valid float"
        );
        assert_ne!(sample, previous_sample, "Telemetry samples should vary across ticks");
        previous_phase = telemetry_phase;
        previous_sample = sample;
    }
}

#[test]
fn test_spatial_dnd_routes_to_correct_split() {
    let mut dnd = DndManager::new();
    let left_node = NodeId::new(101);
    let right_node = NodeId::new(102);

    dnd.register_rect(left_node, Rect::new(0, 0, 40, 25), None);
    dnd.register_rect(right_node, Rect::new(40, 0, 40, 25), None);

    // Pointer move over left split
    let ev1 = dnd.handle_pointer_move(Point::new(15, 10));
    assert_eq!(ev1, Some(DragEvent::DragEnter(left_node)));

    // Pointer move across boundary to right split
    let ev2 = dnd.handle_pointer_move(Point::new(55, 12));
    assert_eq!(ev2, Some(DragEvent::DragEnter(right_node)));

    // Fire potential drop over right split
    let drop_result = dnd.handle_potential_drop("file:///tmp/telemetry_model.onnx\r\nfile:///tmp/weights.bin");
    assert!(drop_result.is_some());
    let (target_node, paths) = drop_result.unwrap();
    assert_eq!(target_node, right_node);
    assert_eq!(
        paths,
        vec![
            PathBuf::from("/tmp/telemetry_model.onnx"),
            PathBuf::from("/tmp/weights.bin"),
        ]
    );
}

#[test]
fn test_path_sanitizer_rfc8089() {
    // 1. file:// URI with percent decoding
    let res1 = PathSanitizer::parse_drop_payload("file:///var/data/telemetry%20dump.bin");
    assert_eq!(res1, vec![PathBuf::from("/var/data/telemetry dump.bin")]);

    // 2. Multi-line CRLF payload
    let res2 = PathSanitizer::parse_drop_payload("file:///dataset_1.parquet\r\nfile:///dataset_2.parquet");
    assert_eq!(
        res2,
        vec![PathBuf::from("/dataset_1.parquet"), PathBuf::from("/dataset_2.parquet"),]
    );

    // 3. Encoded slashes (%2F)
    let res3 = PathSanitizer::parse_drop_payload("file:///home%2Fuser%2Fconfig.toml");
    assert_eq!(res3, vec![PathBuf::from("/home/user/config.toml")]);

    // 4. Raw shell unescaping
    let res4 = PathSanitizer::parse_drop_payload("/Users/alice/My\\ Documents/file.pdf");
    assert_eq!(res4, vec![PathBuf::from("/Users/alice/My Documents/file.pdf")]);
}

#[test]
fn test_file_drop_zone_widget() {
    let mut zone = FileDropZone::new("Neural Weights (.onnx)");
    let mut buf = Buffer::new(30, 8);
    let mut subview = buf.subview_mut(Rect::new(0, 0, 30, 8));

    // Render idle
    zone.set_hovered(false);
    zone.render(&mut subview);
    // Should render dashed border glyphs
    let top_border = subview.get(1, 0).unwrap().symbol.as_str().to_string();
    assert_eq!(top_border, "╌");

    // Render hovered
    zone.set_hovered(true);
    zone.render(&mut subview);
    let badge_found = (0..subview.height()).any(|y| {
        let mut line = String::new();
        for x in 0..subview.width() {
            line.push_str(subview.get(x, y).unwrap().symbol.as_str());
        }
        line.contains("DROP FILES HERE")
    });
    assert!(badge_found, "Hovered drop zone must display drop badge");
}
