use tenui_anim::{ActiveTicker, AnalyticalSpring, MicroStepper, SpringConfig};

#[test]
fn test_active_ticker_lifecycle() {
    let ticker = ActiveTicker::new();
    assert_eq!(ticker.active_count(), 0);
    assert!(!ticker.is_active());
    // Zero-idle sleep state returns None
    assert_eq!(ticker.tick(), None);

    // Register active handles
    assert_eq!(ticker.register_active(), 1);
    assert!(ticker.is_active());

    // Advance tick returns valid clamped delta time
    let dt = ticker.tick().expect("active ticker ticks");
    assert!(dt > 0.0 && dt <= ActiveTicker::MAX_DELTA_TIME_SECS);

    // Register 2nd handle
    assert_eq!(ticker.register_active(), 2);
    assert_eq!(ticker.active_count(), 2);

    // Unregister both
    assert_eq!(ticker.unregister_active(), 1);
    assert!(ticker.is_active());

    assert_eq!(ticker.unregister_active(), 0);
    assert!(!ticker.is_active());
    assert_eq!(ticker.tick(), None, "Ticker drops back to sleep when count is 0");
}

#[test]
fn test_analytical_spring_critical_damping() {
    let mut spring = AnalyticalSpring::new(0.0, SpringConfig::CRITICAL);
    assert_eq!(spring.value(), 0.0);
    assert!(spring.is_settled());

    spring.set_target(100.0);
    assert!(!spring.is_settled());
    assert_eq!(spring.target(), 100.0);

    // Step through simulation
    let dt = 0.016; // 60 FPS
    let mut steps = 0;
    while !spring.is_settled() && steps < 300 {
        let (val, settled) = spring.advance(dt);
        // Critical damping must not significantly overshoot target (> 100.1)
        assert!(val <= 100.1, "Critically damped spring must not overshoot");
        if settled {
            break;
        }
        steps += 1;
    }

    assert!(spring.is_settled(), "Spring must settle within 300 ticks");
    assert!((spring.value() - 100.0).abs() < 0.01);
}

#[test]
fn test_analytical_spring_bouncy_and_velocity_injection() {
    let mut spring = AnalyticalSpring::new(0.0, SpringConfig::BOUNCY);
    spring.set_target(50.0);

    // Inject flick velocity
    spring.inject_velocity(20.0);
    assert!(!spring.is_settled());

    let dt = 0.016;
    let mut saw_overshoot = false;
    for _ in 0..200 {
        let (val, settled) = spring.advance(dt);
        if val > 50.05 {
            saw_overshoot = true;
        }
        if settled {
            break;
        }
    }

    assert!(saw_overshoot, "Bouncy spring must exhibit oscillatory overshoot");
    assert!(spring.is_settled());
    assert!((spring.value() - 50.0).abs() < 0.01);
}

#[test]
fn test_interruption_invariance() {
    let mut spring = AnalyticalSpring::new(0.0, SpringConfig::CRITICAL);
    spring.set_target(100.0);

    // Advance 5 frames (mid-flight)
    for _ in 0..5 {
        spring.advance(0.016);
    }
    let mid_flight_val = spring.value();
    assert!(mid_flight_val > 0.0 && mid_flight_val < 100.0);

    // Abruptly reverse target to 0.0 mid-flight
    spring.set_target(0.0);
    assert!(!spring.is_settled());

    // Position at epoch t=0 must be perfectly continuous (zero jump)
    let immediately_after = spring.value();
    assert!(
        (immediately_after - mid_flight_val).abs() < 1e-4,
        "Position at t=0 of target reversal must exactly equal mid_flight_val"
    );

    // Advance simulation towards 0
    for _ in 0..300 {
        if spring.advance(0.016).1 {
            break;
        }
    }
    assert!(spring.is_settled());
    assert!(spring.value().abs() < 0.01);
}

#[test]
fn test_fractional_micro_stepping() {
    // 0.0 -> 0 whole cells, no fractional glyph
    let step0 = MicroStepper::resolve_horizontal(0.0);
    assert_eq!(step0.whole_cells, 0);
    assert_eq!(step0.fractional_glyph, None);

    // 0.25 (2/8th) -> '▎'
    let step_2_8 = MicroStepper::resolve_horizontal(0.25);
    assert_eq!(step_2_8.whole_cells, 0);
    assert_eq!(step_2_8.fractional_glyph, Some('▎'));

    // 0.5 (4/8th) -> '▌'
    let step_4_8 = MicroStepper::resolve_horizontal(0.5);
    assert_eq!(step_4_8.whole_cells, 0);
    assert_eq!(step_4_8.fractional_glyph, Some('▌'));

    // 3.75 (6/8th) -> 3 whole cells + '▊'
    let step_3_6 = MicroStepper::resolve_horizontal(3.75);
    assert_eq!(step_3_6.whole_cells, 3);
    assert_eq!(step_3_6.fractional_glyph, Some('▊'));

    // Vertical 0.5 (4/8th) -> '▄'
    let vert_4_8 = MicroStepper::resolve_vertical(0.5);
    assert_eq!(vert_4_8.whole_cells, 0);
    assert_eq!(vert_4_8.fractional_glyph, Some('▄'));
}
