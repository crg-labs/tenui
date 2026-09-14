//! # Tenui Showcase — the single canonical example
//!
//! One real, interactive TUI that pushes the framework hard and exercises the breadth of
//! Tenui: the retained reactive `App` reactor, the two-lane engine, the sub-cell
//! compositor, kinetics, the enterprise widget suite, VFX, clipboard, accessibility,
//! collaboration/CRDT, SIMD diffing, math typesetting, media, HID, and chaos fuzzing.
//!
//! ## Run
//!
//! Interactive: `cargo run --release --example showcase`.
//! Tab / Shift-Tab or 1..6 switch tabs · arrows/PgUp/PgDn scroll · Ctrl+C/X/V clipboard ·
//! `p` burst particles · `/` search (Editor) · `?` help modal · `q` / Esc quits.
//!
//! Headless stress + profiling (no TTY needed — for `cargo flamegraph`):
//! `cargo run --release --example showcase -- --bench 2000`.
//! Drives every subsystem each frame under a 100k-row table, max particles, SIMD churn,
//! malformed-input chaos fuzzing, and resize storms — asserting zero panics.
//!
//! Immediate mode is used as the driver because this is a canvas/widget-heavy showcase;
//! the retained `App` reactor is exercised live in the System tab and hammered in bench mode.

use std::{
    io::{self, Write},
    path::PathBuf,
    time::{Duration, Instant},
};

use tenui::{
    anim::{ActiveTicker, AnalyticalSpring, SpringConfig, stepper::MicroStepper},
    chaos::ChaosStreamFuzzer,
    collab::{CollaborativeText, MultiSeatManager, SeatPresence},
    compositor::{
        braille::BrailleCanvas,
        halfblock::HalfBlockCanvas,
        quadrant::QuadrantCanvas,
        spatial_nav::{Direction, FocusNode, SpatialNav},
    },
    core::{
        Announcer, InputEvent, NetworkBandwidthState, PathSanitizer, Role, ScreenReaderBridge, SemanticNode,
        SemanticTree, SgrMouseParser, SshFlowController, a11y, bidi_reorder, format_osc8_hyperlink,
        input::InputDemuxer,
    },
    crossterm::{
        event::{
            self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture, Event, KeyCode,
            KeyModifiers, MouseEventKind,
        },
        execute,
    },
    ext::MiddlewareContext,
    hid::RotaryImpulseController,
    math::{MathNode, MathTypesetter},
    media::{HalfBlockVideoConverter, VideoFrame},
    prelude::*,
    runtime::App,
    simd::SimdDiffScanner,
    std_widgets::{
        chart::{BarChart, BarItem, Candle, CandlestickChart, Heatmap, LinePlot, PlotCanvasType, Series},
        diag::{Diagnostic, DiagnosticView},
        doc::MarkdownView,
        file_picker::FilePicker,
        viz::Sparkline,
        which_key::WhichKey,
    },
    vfx::{
        border::{BorderCompositor, BorderCorner, FrameConfig},
        button::TactileButton,
        particles::ParticlePool,
        shadow::DropShadow,
        spinner::VectorArcSpinner,
        text::{TextEffect, TextOpticsCompositor},
    },
};

// Data model

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Dashboard,
    Data,
    Editor,
    Collab,
    Frontier,
    System,
}
impl Tab {
    const ALL: [Tab; 6] = [
        Tab::Dashboard,
        Tab::Data,
        Tab::Editor,
        Tab::Collab,
        Tab::Frontier,
        Tab::System,
    ];
    fn title(&self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Data => "Data",
            Tab::Editor => "Editor",
            Tab::Collab => "Collab",
            Tab::Frontier => "Frontier",
            Tab::System => "System",
        }
    }
    fn index(&self) -> usize {
        Tab::ALL.iter().position(|t| t == self).unwrap_or(0)
    }
}

/// A process row for the 100k-row VirtualTable stress test.
#[derive(Clone)]
struct Proc {
    pid: u32,
    name: String,
    cpu: f32,
    mem_mb: f32,
}

/// Tiny deterministic PRNG (no rand dependency).
struct Rng(u64);
impl Rng {
    fn u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn f32(&mut self) -> f32 {
        (self.u32() % 10_000) as f32 / 10_000.0
    }
    fn range(&mut self, n: usize) -> usize {
        (self.u32() as usize) % n.max(1)
    }
}

struct State {
    theme: ThemePalette,
    tab: Tab,
    frame: u64,
    t: f32, // seconds
    rng: Rng,

    // animation
    ticker: ActiveTicker,
    pulse: AnalyticalSpring,
    particles: ParticlePool,

    // dashboard data
    cpu_hist: Vec<(f64, f64)>,
    ram: (f32, f32, f32),
    spark: Vec<u64>,
    multi: MultiProgress,
    heat: Vec<Vec<f64>>,

    // data tab
    procs: Vec<Proc>,
    table_scroll: usize,
    tree: TreeView<String>,
    file_picker: FilePicker,

    // editor tab
    editor: TextEditor,
    search: String,
    diag_view: bool,
    show_markdown: bool,
    markdown: MarkdownView,
    diagnostics: Vec<Diagnostic>,

    // frontier
    candles: Vec<Candle>,
    ssh: SshFlowController,
    session_ron_ok: bool,

    // overlays
    which_key: WhichKey,
    show_which: bool,

    // collab tab
    chat: ChatThread,
    collab: CollaborativeText,
    seats: MultiSeatManager,
    repl: ReplInput,

    // frontier
    rotary: RotaryImpulseController,
    video: VideoFrame,

    // system tab
    reactor: App<Vec<u8>>,
    reactor_body: u64,
    spatial: SpatialNav,
    focus_nodes: Vec<FocusNode>,
    focus_id: u64,
    announcer: Announcer,

    // logs / status / io
    log: LogStreamer,
    clipboard: Clipboard,
    status: String,
    show_help: bool,
    show_palette: bool,
    palette: CommandPalette,
    dispatch: MiddlewarePipeline,
    dropped_frames: u64,
    quit: bool,
}

fn build_state(width: u16, height: u16) -> State {
    let theme = ThemePalette::tokyo_night();
    let mut rng = Rng(0xA11CE);

    // 100k processes — the virtualization stress test.
    let procs: Vec<Proc> = (0..100_000)
        .map(|i| Proc {
            pid: 1000 + i as u32,
            name: format!("proc-{:05}", i),
            cpu: rng.f32() * 100.0,
            mem_mb: rng.f32() * 4096.0,
        })
        .collect();

    let tree = TreeView::new(vec![TreeNode::new(0, "workspace", "root".into()).with_children(vec![
        TreeNode::new(1, "src", "dir".into()).with_children(vec![
            TreeNode::new(2, "main.rs", "file".into()),
            TreeNode::new(3, "lib.rs", "file".into()),
        ]),
        TreeNode::new(4, "Cargo.toml", "file".into()),
    ])]);

    let mut multi = MultiProgress::new();
    for name in ["Render", "SIMD diff", "CRDT sync", "Chaos fuzz"] {
        multi.add_task(name);
    }

    let mut chat = ChatThread::new();
    chat.add_message(ChatMessage::user("Push Tenui to the extreme."));
    chat.add_message({
        let mut m = ChatMessage::assistant_streaming();
        m.append_token("Streaming a response, rendering charts, running the CRDT…");
        m.is_streaming = false;
        m
    });

    let mut collab = CollaborativeText::new(1);
    let _ = collab.insert("collaborative buffer — type together");

    let mut seats = MultiSeatManager::with_site(1);
    seats.add_seat(SeatPresence {
        seat_id: 2,
        name: "Bob".into(),
        cursor_pos: Point::new(8, 1),
        accent_color: theme.warning,
        active_pane: 0,
    });

    // A small procedural RGB video frame (checker) for the media converter.
    let (vw, vh) = (48u16, 32u16);
    let mut vdata = Vec::with_capacity(vw as usize * vh as usize * 3);
    for y in 0..vh {
        for x in 0..vw {
            let on = (x / 4 + y / 4).is_multiple_of(2);
            let (r, g, b) = if on { (30, 120, 200) } else { (10, 20, 40) };
            vdata.extend_from_slice(&[r, g, b]);
        }
    }
    let video = VideoFrame::new(vw, vh, tenui::media::PixelFormat::Rgb8, vdata, Duration::ZERO);

    // Retained reactor for the System tab: a header + body + footer column.
    let mut reactor = App::new(Terminal::with_writer(Vec::new(), width, height));
    let root = reactor.root();
    let leaf_style = || tenui::taffy::Style {
        size: tenui::taffy::Size {
            width: tenui::taffy::prelude::percent(1.0),
            height: tenui::taffy::prelude::length(3.0),
        },
        ..Default::default()
    };
    let _h = reactor.add_leaf(root, leaf_style(), |sv| {
        sv.write_str_clipped(0, 0, "reactor header", Color::White, Color::Reset);
    });
    let reactor_body = reactor.add_leaf(root, leaf_style(), |sv| {
        sv.write_str_clipped(
            0,
            0,
            "reactor body (paint-only frames skip Taffy)",
            Color::Cyan,
            Color::Reset,
        );
    });
    reactor.set_semantic(reactor_body, Role::Group, "reactor body");

    let focus_nodes = vec![
        FocusNode::new(1, Rect::new(2, 2, 10, 3)),
        FocusNode::new(2, Rect::new(20, 2, 10, 3)),
        FocusNode::new(3, Rect::new(2, 8, 10, 3)),
        FocusNode::new(4, Rect::new(20, 8, 10, 3)),
    ];

    let cmd = |id: &str, title: &str, cat: &str| CommandItem {
        id: id.into(),
        title: title.into(),
        category: cat.into(),
    };
    let palette = CommandPalette::new(vec![
        cmd("dash", "Go: Dashboard", "nav"),
        cmd("data", "Go: Data", "nav"),
        cmd("help", "Toggle Help", "view"),
        cmd("quit", "Quit", "app"),
    ]);

    let dispatch = MiddlewarePipeline::new().with_middleware(tenui::ext::ClipboardLayer::new());

    let diagnostics = vec![
        Diagnostic::error(Some("E0384"), "cannot assign twice to immutable variable `x`")
            .with_source("main.rs", SAMPLE_CODE)
            .with_help("consider making this binding mutable: `let mut x`"),
        Diagnostic::warning(Some("unused_variables"), "unused variable: `app`")
            .with_note("`#[warn(unused_variables)]` on by default"),
    ];
    let markdown = MarkdownView::new(
        "# Tenui\n\nA **fast, lightweight** TUI framework.\n\n- retained two-lane engine\n- sub-cell compositor\n- `#![forbid(unsafe_code)]` everywhere but SIMD\n",
    );
    let candles: Vec<Candle> = (0..24)
        .map(|i| {
            let base = 50.0 + 20.0 * (i as f64 * 0.5).sin();
            Candle {
                open: base,
                high: base + 6.0,
                low: base - 5.0,
                close: base + 2.0 * ((i % 3) as f64 - 1.0),
                volume: 100.0,
            }
        })
        .collect();
    let mut which_key = WhichKey::new("Leader");
    which_key.add_action("f", "find file");
    which_key.add_action("g", "goto");
    which_key.add_action("p", "command palette");
    which_key.add_action("q", "quit");

    State {
        theme,
        tab: Tab::Dashboard,
        frame: 0,
        t: 0.0,
        rng,
        ticker: ActiveTicker::new(),
        pulse: AnalyticalSpring::new(0.0, SpringConfig::BOUNCY),
        particles: ParticlePool::new(),
        cpu_hist: (0..120).map(|i| (i as f64, 20.0)).collect(),
        ram: (2048.0, 1024.0, 1024.0),
        spark: vec![3, 5, 8, 6, 9, 4, 7, 10, 6, 8, 5, 9, 12, 7, 4],
        multi,
        heat: (0..8)
            .map(|r| (0..16).map(|c| ((r * c) as f64).sin().abs()).collect())
            .collect(),
        procs,
        table_scroll: 0,
        tree,
        editor: TextEditor::from_text(SAMPLE_CODE),
        search: String::new(),
        diag_view: false,
        chat,
        collab,
        seats,
        repl: ReplInput::new(),
        rotary: RotaryImpulseController::new(2.0),
        video,
        reactor,
        reactor_body,
        spatial: SpatialNav::new(),
        focus_nodes,
        focus_id: 1,
        announcer: Announcer::new(),
        log: LogStreamer::new(2000),
        clipboard: Clipboard::new(),
        status: "Ready — Tenui showcase".into(),
        show_help: false,
        show_palette: false,
        palette,
        dispatch,
        dropped_frames: 0,
        quit: false,
        file_picker: FilePicker::new(PathBuf::from(".")),
        show_markdown: false,
        markdown,
        diagnostics,
        candles,
        ssh: SshFlowController::new(),
        session_ron_ok: session_roundtrip_ok(),
        which_key,
        show_which: false,
    }
}

/// Serializes a workspace layout to RON and reads it back — exercises tenui-session.
fn session_roundtrip_ok() -> bool {
    use tenui::session::{SerializedDimension, SerializedFlexDirection, SerializedPaneNode, WorkspaceSession};
    let root = SerializedPaneNode {
        id: 1,
        title: "root".into(),
        flex_direction: SerializedFlexDirection::Row,
        flex_grow: 1.0,
        flex_shrink: 1.0,
        size_width: SerializedDimension::Percent(100.0),
        size_height: SerializedDimension::Percent(100.0),
        children: Vec::new(),
        component_type: "Workspace".into(),
        metadata: std::collections::HashMap::new(),
    };
    let session = WorkspaceSession::new(root, (120, 40));
    let ron = session.to_ron();
    WorkspaceSession::from_ron(&ron).map(|s| s == session).unwrap_or(false)
}

const SAMPLE_CODE: &str = "fn main() {\n    let mut app = App::new(terminal);\n    // two-lane: paint-only edits skip Taffy\n    app.render_frame();\n    println!(\"tenui\");\n}\n";

// Update — advance every subsystem each frame (this is the profiled workload).

fn update(state: &mut State, dt: f32) {
    state.frame += 1;
    state.t += dt;
    let t = state.t;

    // Animated telemetry.
    let cpu = 50.0 + 40.0 * (t * 1.3).sin();
    state.cpu_hist.remove(0);
    let x = state.cpu_hist.last().map(|(x, _)| *x + 1.0).unwrap_or(0.0);
    state.cpu_hist.push((x, cpu as f64));
    state.ram.0 = 2048.0 + 512.0 * (t * 0.7).sin();
    state.spark.remove(0);
    state.spark.push((cpu / 8.0) as u64);
    for (i, row) in state.heat.iter_mut().enumerate() {
        for (j, v) in row.iter_mut().enumerate() {
            *v = ((t + i as f32 * 0.3 + j as f32 * 0.2).sin() * 0.5 + 0.5) as f64;
        }
    }

    // Kinetics + particles. The ActiveTicker demand-scheduler is kept "active" while
    // animations run so an idle app can drop to zero-CPU wait.
    if state.frame == 1 {
        state.ticker.register_active();
    }
    let _ = state.ticker.tick();
    let _ = state.pulse.advance(dt);
    state
        .particles
        .spawn_burst(20.0, 6.0, state.rng.f32() * 4.0 - 2.0, -3.0, state.theme.accent, 1.5);
    state.particles.update(dt);
    state.multi.tick_spring_collapse(dt);
    for i in 0..4 {
        if let Some(task) = state.multi.get_task_mut(i) {
            task.progress = ((t * 0.2 + i as f32 * 0.25) % 1.0).abs();
        }
    }

    // Streaming logs (LogStreamer ring arena).
    state.log.push_line(format!(
        "[{:>6}] frame tick · cpu={:.1}% · particles={}",
        state.frame, cpu, state.particles.count
    ));

    // Collaboration: a remote peer types occasionally (converges via CRDT).
    if state.frame.is_multiple_of(20) {
        // Simulate a second replica's op by inserting locally (demonstration).
        let _ = state.collab.insert(".");
    }
    state.rotary.step(dt);

    // SSH flow controller reacts to (simulated) network latency, driving adaptive degradation.
    let latency_ms = 20.0 + 60.0 * (t * 0.4).sin().max(0.0);
    state.ssh.record_latency(latency_ms);
}

// Render

fn render(canvas: &mut CanvasSubviewMut<'_>, state: &mut State) {
    let w = canvas.width();
    let h = canvas.height();
    if w < 20 || h < 8 {
        return;
    }
    let th = state.theme.clone();
    canvas.clear(th.bg);

    // Header (tab bar).
    {
        let mut hdr = canvas.subview_mut(Rect::new(0, 0, w, 2));
        hdr.clear(th.surface);
        let mut x = 1u16;
        for tab in Tab::ALL {
            let active = tab == state.tab;
            let label = format!(" {} {} ", tab.index() + 1, tab.title());
            let (fg, bg) = if active {
                (th.bg, th.primary)
            } else {
                (th.fg_muted, th.surface)
            };
            hdr.write_str_clipped(x, 0, &label, fg, bg);
            x += label.chars().count() as u16 + 1;
        }
        let right = format!("tenui · {}  ", th.name);
        let rx = w.saturating_sub(right.chars().count() as u16);
        hdr.write_str_clipped(rx, 0, &right, th.accent, th.surface);
    }

    // Body.
    let body = Rect::new(0, 2, w, h.saturating_sub(3));
    {
        let mut b = canvas.subview_mut(body);
        match state.tab {
            Tab::Dashboard => render_dashboard(&mut b, state),
            Tab::Data => render_data(&mut b, state),
            Tab::Editor => render_editor(&mut b, state),
            Tab::Collab => render_collab(&mut b, state),
            Tab::Frontier => render_frontier(&mut b, state),
            Tab::System => render_system(&mut b, state),
        }
    }

    // Footer.
    {
        let mut ft = canvas.subview_mut(Rect::new(0, h - 1, w, 1));
        ft.clear(th.surface);
        let hint = "Tab/1-6 switch · arrows scroll · Ctrl+C/V clip · p burst · / search · ? help · q quit";
        ft.write_str_clipped(1, 0, hint, th.fg_muted, th.surface);
        let st = format!(" {} ", state.status);
        let sx = w.saturating_sub(st.chars().count() as u16);
        ft.write_str_clipped(sx, 0, &st, th.bg, th.accent);
    }

    // Overlays.
    if state.show_palette {
        let pw = (w * 6 / 10).max(20);
        let ph = 8u16.min(h - 4);
        let px = (w - pw) / 2;
        let mut ov = canvas.subview_mut(Rect::new(px, 3, pw, ph));
        DropShadow::ELEVATED.render_shadow(&mut ov, Rect::new(0, 0, pw, ph));
        state.palette.render(&mut ov);
    }
    if state.show_which {
        let kw = (w * 4 / 10).max(20);
        let kh = 7u16.min(h - 4);
        let mut ov = canvas.subview_mut(Rect::new(1, h.saturating_sub(kh + 1), kw, kh));
        state.which_key.render(&mut ov);
    }
    if state.show_help {
        let mw = (w * 5 / 10).max(30);
        let mh = 10u16.min(h - 4);
        let mx = (w - mw) / 2;
        let modal = Modal::new("Help", mw, mh, 99).with_border_color(th.accent);
        let mut ov = canvas.subview_mut(Rect::new(mx, 3, mw, mh));
        modal.render(&mut ov, |c| {
            for (i, line) in [
                "Tenui showcase — every subsystem, one app.",
                "",
                "Tab / 1-6   switch tabs",
                "arrows      scroll / navigate",
                "Ctrl+C/X/V  clipboard (Editor)",
                "p           particle burst",
                "?           toggle this help",
                "q / Esc     quit",
            ]
            .iter()
            .enumerate()
            {
                c.write_str_clipped(1, i as u16, line, th.fg, th.bg);
            }
        });
    }
}

fn render_dashboard(c: &mut CanvasSubviewMut<'_>, state: &mut State) {
    let th = state.theme.clone();
    let w = c.width();
    let h = c.height();
    let half = w / 2;

    // CPU line plot (braille) + sparkline.
    {
        let mut p = c.subview_mut(Rect::new(1, 0, half.saturating_sub(2), h / 2));
        LinePlot::new()
            .add_series(Series::new("CPU %", th.primary, state.cpu_hist.clone()))
            .with_canvas_type(PlotCanvasType::Braille)
            .render(&mut p);
    }
    // RAM bar chart.
    {
        let mut bc = c.subview_mut(Rect::new(1, h / 2, half.saturating_sub(2), h / 2 - 1));
        BarChart::new(vec![
            BarItem {
                label: "Used".into(),
                value: state.ram.0 as f64,
                color: th.warning,
            },
            BarItem {
                label: "Cache".into(),
                value: state.ram.1 as f64,
                color: th.accent,
            },
            BarItem {
                label: "Free".into(),
                value: state.ram.2 as f64,
                color: th.success,
            },
        ])
        .render(&mut bc);
    }
    // Right column: heatmap, multiprogress, particles, spinner, vfx frame.
    {
        let mut hm = c.subview_mut(Rect::new(half, 0, w - half - 1, h / 3));
        Heatmap::new(state.heat.clone()).render(&mut hm);
    }
    {
        let mut mp = c.subview_mut(Rect::new(half, h / 3, w - half - 1, h / 3));
        state.multi.render(&mut mp, th.fg, th.bg);
    }
    {
        // Particle fountain + a spinning vector arc + a tactile button, over a frame.
        let region = Rect::new(half, 2 * h / 3, w - half - 1, h / 3 - 1);
        let mut fx = c.subview_mut(region);
        BorderCompositor::render_frame(
            &mut fx,
            &FrameConfig {
                corner: BorderCorner::Chamfered,
                border_color: th.accent,
                background: None,
                badge_cutout: Some("[ VFX ]".into()),
            },
        );
        state.particles.render(&mut fx, true);
        VectorArcSpinner::new(3.0, th.primary).render_arc(&mut fx, state.t);
        TactileButton::new("Deploy", th.success).render(&mut fx.subview_mut(Rect::new(
            2,
            region.height.saturating_sub(3),
            14,
            3,
        )));
    }
    // Kinetic shimmer title + a live sparkline, overlaid on the top row.
    TextOpticsCompositor::render_text(
        c,
        1,
        0,
        "◆ TERMINUS",
        th.primary,
        TextEffect::Shimmer {
            speed: 2.0,
            peak_color: th.accent,
        },
        state.t,
    );
    {
        let mut sp = c.subview_mut(Rect::new(half, 0, (w - half).saturating_sub(1), 1));
        Sparkline::new(&state.spark).render(&mut sp, th.success, th.bg);
    }
}

fn render_data(c: &mut CanvasSubviewMut<'_>, state: &mut State) {
    let th = state.theme.clone();
    let w = c.width();
    let h = c.height();
    let split = (w * 7) / 10;
    // 100k-row virtual table (only the viewport is materialized).
    {
        let mut tv = c.subview_mut(Rect::new(1, 0, split.saturating_sub(2), h));
        let mut table = VirtualTable::new(
            &state.procs,
            vec![
                Column::new("PID", 8, |p: &Proc| p.pid.to_string()),
                Column::new("NAME", 16, |p: &Proc| p.name.clone()),
                Column::new("CPU%", 8, |p: &Proc| format!("{:.1}", p.cpu)),
                Column::new("MEM MB", 10, |p: &Proc| format!("{:.0}", p.mem_mb)),
            ],
        );
        table.scroll_offset = state.table_scroll;
        table.render(&mut tv);
        tv.write_str_clipped(
            0,
            h - 1,
            &format!(" {} rows · row {} ", state.procs.len(), state.table_scroll),
            th.bg,
            th.accent,
        );
    }
    // Tree (top) + mime-aware FilePicker (bottom).
    {
        let mut tr = c.subview_mut(Rect::new(split, 0, w - split - 1, h / 2));
        state.tree.render(&mut tr);
    }
    {
        let mut fp = c.subview_mut(Rect::new(split, h / 2, w - split - 1, h - h / 2));
        state.file_picker.render(&mut fp, th.fg, th.bg);
    }
}

fn render_editor(c: &mut CanvasSubviewMut<'_>, state: &mut State) {
    let th = state.theme.clone();
    let w = c.width();
    let h = c.height();
    let half = w / 2;
    let diag_h = 4u16.min(h / 3);
    let edit_h = h.saturating_sub(diag_h);
    // Live editor (shared TextBuffer core: selection, undo, clipboard, search). The 1/8th
    // MicroStepper glyph shows sub-cell fractional motion.
    {
        let ms = MicroStepper::resolve_horizontal((state.t.fract()) * 8.0);
        let glyph = ms.fractional_glyph.unwrap_or(' ');
        let mut ed = c.subview_mut(Rect::new(1, 1, half.saturating_sub(2), edit_h.saturating_sub(1)));
        ed.write_str_clipped(
            0,
            0,
            &format!(
                "Editor {}  (search: {})",
                glyph,
                if state.search.is_empty() { "—" } else { &state.search }
            ),
            th.accent,
            th.bg,
        );
        let mut body = ed.subview_mut(Rect::new(0, 1, half.saturating_sub(2), edit_h.saturating_sub(2)));
        state.editor.render(&mut body, true);
    }
    // Right panel: Markdown ('m') · Diff ('d') · syntax-highlighted CodeView.
    {
        let mut cv = c.subview_mut(Rect::new(half, 1, w - half - 1, edit_h.saturating_sub(1)));
        if state.show_markdown {
            state.markdown.render(&mut cv);
        } else if state.diag_view {
            DiffView::new(SAMPLE_CODE, state.editor.text()).render(&mut cv);
        } else {
            CodeView::new(SAMPLE_CODE, Language::Rust).render(&mut cv);
        }
    }
    // Compiler-grade diagnostics strip along the bottom.
    {
        let mut dv = c.subview_mut(Rect::new(1, edit_h, w.saturating_sub(2), diag_h));
        DiagnosticView::new(state.diagnostics.clone()).render(&mut dv);
    }
}

fn render_collab(c: &mut CanvasSubviewMut<'_>, state: &mut State) {
    let th = state.theme.clone();
    let w = c.width();
    let h = c.height();
    let half = w / 2;
    // Streaming chat thread.
    {
        let mut ch = c.subview_mut(Rect::new(1, 0, half.saturating_sub(2), h - 1));
        state.chat.render(&mut ch, th.bg);
    }
    // CRDT collaborative buffer + remote carets + repl.
    {
        let mut cb = c.subview_mut(Rect::new(half, 0, w - half - 1, h));
        cb.write_str_clipped(0, 0, "CRDT buffer (convergent):", th.accent, th.bg);
        let text = state.collab.text();
        cb.write_str_clipped(0, 1, &text, th.fg, th.bg);
        let mut seat_area = cb.subview_mut(Rect::new(0, 3, w - half - 1, 4));
        state.seats.render_remote_seats(1, &mut seat_area);
        let mut rp = cb.subview_mut(Rect::new(0, h - 3, w - half - 1, 3));
        state.repl.render_at(&mut rp, 0, th.fg, th.bg, "> ");
    }
}

fn render_frontier(c: &mut CanvasSubviewMut<'_>, state: &mut State) {
    let th = state.theme.clone();
    let w = c.width();
    let h = c.height();
    let half = w / 2;
    // Media: procedural video via half-block converter.
    {
        let mut vid = c.subview_mut(Rect::new(1, 0, half.saturating_sub(2), h - 1));
        vid.write_str_clipped(0, 0, "Half-block video (2px/cell):", th.accent, th.bg);
        let mut frame = vid.subview_mut(Rect::new(0, 1, (state.video.width).min(half), h - 2));
        HalfBlockVideoConverter::render_rgb(
            &mut frame,
            0,
            0,
            state.video.width,
            state.video.height,
            &state.video.data,
        );
    }
    // Math typesetting + HID rotary + SIMD stat.
    {
        let mut r = c.subview_mut(Rect::new(half, 0, w - half - 1, h));
        r.write_str_clipped(0, 0, "Sub-cell math typesetting:", th.accent, th.bg);
        let expr = MathNode::Fraction(
            Box::new(MathNode::Text("dv".into())),
            Box::new(MathNode::Text("dt".into())),
        );
        MathTypesetter.render_themed(&mut r, 0, 2, &expr, th.fg, th.bg);
        r.write_str_clipped(
            0,
            5,
            &format!("HID rotary v = {:.2} rad/s", state.rotary.current_velocity),
            th.fg,
            th.bg,
        );
        r.write_str_clipped(0, 6, &format!("SIMD backend: {}", simd_backend()), th.fg_muted, th.bg);
        let (tier, tcol) = match state.ssh.state() {
            NetworkBandwidthState::Nominal => ("Nominal (60fps TrueColor)", th.success),
            NetworkBandwidthState::Degraded => ("Degraded (30fps ANSI256)", th.warning),
            NetworkBandwidthState::Congested => ("Congested (15fps ANSI16)", th.error),
        };
        r.write_str_clipped(0, 7, &format!("SSH flow: {}", tier), tcol, th.bg);
        r.write_str_clipped(
            0,
            8,
            &format!(
                "session RON round-trip: {}",
                if state.session_ron_ok { "ok" } else { "FAIL" }
            ),
            th.fg_muted,
            th.bg,
        );
        if h > 12 {
            let mut cs = r.subview_mut(Rect::new(0, 10, w - half - 1, h - 10));
            CandlestickChart::new(state.candles.clone()).render(&mut cs);
        }
    }
}

fn render_system(c: &mut CanvasSubviewMut<'_>, state: &mut State) {
    let th = state.theme.clone();
    let w = c.width();
    let h = c.height();
    let half = w / 2;

    // Live retained reactor: run one frame and report two-lane metrics.
    state.reactor.mark_paint(state.reactor_body);
    let report = state.reactor.render_frame().unwrap_or(FrameReport {
        relayout: false,
        painted: 0,
    });
    {
        let mut rr = c.subview_mut(Rect::new(1, 0, half.saturating_sub(2), h / 2));
        rr.write_str_clipped(0, 0, "Retained reactor (tenui-runtime):", th.accent, th.bg);
        rr.write_str_clipped(
            0,
            2,
            &format!("Taffy reflow passes:  {}", state.reactor.reflow_passes()),
            th.fg,
            th.bg,
        );
        rr.write_str_clipped(
            0,
            3,
            &format!("node paint ops:       {}", state.reactor.paint_ops()),
            th.fg,
            th.bg,
        );
        rr.write_str_clipped(
            0,
            4,
            &format!("this frame relayout:  {}", report.relayout),
            th.fg,
            th.bg,
        );
        rr.write_str_clipped(0, 5, "^ paint-only frames skip Taffy reflow", th.success, th.bg);
        rr.write_str_clipped(
            0,
            7,
            &format!("ActiveTicker: {} active worker(s)", state.ticker.active_count()),
            th.fg_muted,
            th.bg,
        );
        // Drain live-region announcements (Announcer) into a status line.
        if let Some(a) = state.announcer.drain().last() {
            rr.write_str_clipped(0, 8, &format!("announce: {}", a.text), th.warning, th.bg);
        }
        // BiDi reordering + OSC 8 hyperlink emission.
        rr.write_str_clipped(
            0,
            10,
            &format!("BiDi: {}", bidi_reorder("tenui مرحبا 42")),
            th.fg_muted,
            th.bg,
        );
        let link = format_osc8_hyperlink("https://tenui.rs", "docs");
        rr.write_str_clipped(
            0,
            11,
            &format!("OSC 8 link: {} bytes emitted", link.len()),
            th.fg_muted,
            th.bg,
        );
    }

    // Accessibility: semantic tree transcript.
    {
        let mut ax = c.subview_mut(Rect::new(1, h / 2, half.saturating_sub(2), h / 2));
        ax.write_str_clipped(0, 0, "Accessibility transcript:", th.accent, th.bg);
        let tree = sample_semantic_tree();
        for (i, line) in ScreenReaderBridge::describe_tree(&tree)
            .lines()
            .take((h / 2).saturating_sub(1) as usize)
            .enumerate()
        {
            ax.write_str_clipped(1, 1 + i as u16, line, th.fg_muted, th.bg);
        }
    }

    // Sub-cell canvases (braille / half-block / quadrant) + spatial nav focus.
    {
        let mut cv = c.subview_mut(Rect::new(half, 0, w - half - 1, h));
        cv.write_str_clipped(0, 0, "Sub-cell canvases + spatial nav:", th.accent, th.bg);

        let mut braille = BrailleCanvas::new((w - half) / 2, 6);
        for i in 0..40 {
            let y = (3.0 + 2.5 * (state.t + i as f32 * 0.3).sin()) as i32;
            braille.set_dot(i, y.max(0) as u16);
        }
        braille.render_to_subview(
            &mut cv.subview_mut(Rect::new(0, 1, (w - half) / 2, 6)),
            th.primary,
            th.bg,
        );

        let mut hb = HalfBlockCanvas::new((w - half) / 2, 6);
        hb.draw_line_aa(0.0, 0.0, ((w - half) / 2) as f32, 11.0, (240, 100, 200));
        hb.render_to_subview(&mut cv.subview_mut(Rect::new((w - half) / 2, 1, (w - half) / 2, 6)));

        let mut quad = QuadrantCanvas::new(w - half - 1, 4);
        for i in 0..((w - half - 1) * 2) {
            quad.set_pixel(i, (2.0 + 2.0 * (state.t * 2.0 + i as f32 * 0.2).cos()) as u16);
        }
        quad.render_to_subview(&mut cv.subview_mut(Rect::new(0, 8, w - half - 1, 4)), th.accent, th.bg);

        cv.write_str_clipped(
            0,
            13,
            &format!("spatial focus id = {} (WICG k≈2.0)", state.focus_id),
            th.fg_muted,
            th.bg,
        );
    }
}

fn sample_semantic_tree() -> SemanticTree {
    SemanticTree::new(
        SemanticNode::new(0, Role::Dialog, "Showcase")
            .child(SemanticNode::new(1, Role::Tab, "Dashboard"))
            .child({
                let mut b = SemanticNode::new(2, Role::Button, "Deploy");
                b.focused = true;
                b
            })
            .child(SemanticNode::new(3, Role::TextInput, "Editor").with_value("fn main")),
    )
}

fn simd_backend() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        if std::is_x86_feature_detected!("avx2") {
            return "AVX2 (256-bit movemask)";
        }
    }
    "portable unrolled fallback"
}

// Input (interactive)

fn handle_key(state: &mut State, code: KeyCode, mods: KeyModifiers) -> bool {
    // Clipboard chords via the ClipboardLayer middleware → focused editor.
    let ev = InputEvent::Key(tenui::crossterm::event::KeyEvent::new(code, mods));
    let mut cx = MiddlewareContext::default();
    if state.dispatch.dispatch(&ev, &mut cx) == tenui::ext::EventResult::Consumed {
        if let Some(cmd) = cx.clipboard_command {
            if let Some(text) = cx.clipboard.take() {
                state.clipboard.set_register(text);
            }
            if let Some(esc) = tenui::core::apply_clipboard_command(&mut state.editor, cmd, &mut state.clipboard) {
                state.status = format!("clipboard: {} ({} bytes escape)", format_cmd(cmd), esc.len());
            } else {
                state.status = format!("clipboard: {}", format_cmd(cmd));
            }
        }
        return false;
    }

    if state.show_palette {
        if code == KeyCode::Esc {
            state.show_palette = false;
        }
        return false;
    }

    let prev_tab = state.tab;
    match code {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Char('?') => state.show_help = !state.show_help,
        KeyCode::Tab => state.tab = Tab::ALL[(state.tab.index() + 1) % Tab::ALL.len()],
        KeyCode::BackTab => state.tab = Tab::ALL[(state.tab.index() + Tab::ALL.len() - 1) % Tab::ALL.len()],
        KeyCode::Char(d @ '1'..='6') => {
            state.tab = Tab::ALL[(d as usize - '1' as usize).min(5)];
        }
        KeyCode::Char('p') => {
            for _ in 0..40 {
                state.particles.spawn_burst(
                    20.0,
                    6.0,
                    state.rng.f32() * 8.0 - 4.0,
                    -5.0 * state.rng.f32(),
                    state.theme.warning,
                    2.0,
                );
            }
            state.status = "particle burst".into();
        }
        KeyCode::Char('/') if state.tab == Tab::Editor => {
            state.search = "fn".into();
            let matches = state.editor.buffer().find(&state.search, false);
            state.status = format!("search '{}' → {} matches", state.search, matches.len());
        }
        KeyCode::Char('d') if state.tab == Tab::Editor => state.diag_view = !state.diag_view,
        KeyCode::Char('m') if state.tab == Tab::Editor => state.show_markdown = !state.show_markdown,
        KeyCode::Char(' ') if state.tab != Tab::Editor => state.show_which = !state.show_which,
        KeyCode::Down => match state.tab {
            Tab::Data => state.table_scroll = (state.table_scroll + 1).min(state.procs.len().saturating_sub(1)),
            Tab::Editor => state.editor.move_down(false),
            _ => {}
        },
        KeyCode::Up => match state.tab {
            Tab::Data => state.table_scroll = state.table_scroll.saturating_sub(1),
            Tab::Editor => state.editor.move_up(false),
            _ => {}
        },
        KeyCode::PageDown if state.tab == Tab::Data => {
            state.table_scroll = (state.table_scroll + 20).min(state.procs.len().saturating_sub(1));
        }
        KeyCode::PageUp if state.tab == Tab::Data => state.table_scroll = state.table_scroll.saturating_sub(20),
        KeyCode::Left => {
            if let Some(id) = state
                .spatial
                .navigate(state.focus_id, Direction::Left, &state.focus_nodes, None)
            {
                state.focus_id = id;
            }
        }
        KeyCode::Right => {
            if let Some(id) = state
                .spatial
                .navigate(state.focus_id, Direction::Right, &state.focus_nodes, None)
            {
                state.focus_id = id;
            }
        }
        KeyCode::Char(ch) if state.tab == Tab::Editor => state.editor.insert_char(ch),
        KeyCode::Backspace if state.tab == Tab::Editor => state.editor.backspace(),
        KeyCode::Char('P') if mods.contains(KeyModifiers::CONTROL) => state.show_palette = true,
        _ => {}
    }
    if state.tab != prev_tab {
        state.announcer.polite(format!("{} tab", state.tab.title()));
    }
    false
}

fn format_cmd(cmd: tenui::core::ClipboardCommand) -> &'static str {
    use tenui::core::ClipboardCommand::*;
    match cmd {
        Copy => "copy",
        Cut => "cut",
        Paste => "paste",
        SelectAll => "select-all",
    }
}

fn run_interactive() -> io::Result<()> {
    let mut term = Terminal::new()?;
    let (w, h) = term.size();
    let mut out = io::stdout();
    let _ = execute!(out, EnableMouseCapture, EnableBracketedPaste);

    let mut state = build_state(w, h);
    let mut clock = FrameClock::new();
    let frame_budget = Duration::from_millis(16);

    let running = loop {
        let start = Instant::now();

        // Drain input.
        while event::poll(Duration::from_millis(0))? {
            match event::read()? {
                Event::Key(k) => {
                    if handle_key(&mut state, k.code, k.modifiers) {
                        state.quit = true;
                    }
                }
                Event::Paste(s) => {
                    state.clipboard.set_register(s.clone());
                    if state.tab == Tab::Editor {
                        state.editor.buffer_mut().insert(&s);
                    }
                    state.status = "pasted".into();
                }
                Event::Mouse(m) => {
                    if let MouseEventKind::ScrollDown = m.kind {
                        if state.tab == Tab::Data {
                            state.table_scroll = (state.table_scroll + 3).min(state.procs.len() - 1);
                        }
                    } else if let MouseEventKind::ScrollUp = m.kind {
                        state.table_scroll = state.table_scroll.saturating_sub(3);
                    } else if let MouseEventKind::Down(_) = m.kind
                        && m.row <= 1
                    {
                        // Tab bar click (approx): pick by column band.
                        let idx = (m.column as usize / 12).min(Tab::ALL.len() - 1);
                        state.tab = Tab::ALL[idx];
                    }
                }
                Event::Resize(_, _) => term.force_repaint(),
                _ => {}
            }
        }

        let dt = clock.tick().min(0.05);
        update(&mut state, dt);
        state.status = format!(
            "{:.0} fps · frame {} · {}",
            clock.fps(),
            state.frame,
            state.status_tag()
        );

        term.draw(|c| render(c, &mut state))?;

        if state.quit {
            break false;
        }
        if let Some(rem) = frame_budget.checked_sub(start.elapsed()) {
            std::thread::sleep(rem);
        }
    };
    let _ = running;

    let _ = execute!(out, DisableMouseCapture, DisableBracketedPaste);
    Ok(())
}

// Bench / resilience (headless — for cargo flamegraph)

fn run_bench(frames: u64) {
    println!("tenui showcase · bench mode · {} frames", frames);
    let (w, h) = (200u16, 60u16);
    let sink = io::sink();
    let mut term = Terminal::with_writer(sink, w, h);
    let mut state = build_state(w, h);
    let fuzzer = ChaosStreamFuzzer::new();
    let mut demux = InputDemuxer::new();
    let mut rng = Rng(0xBADC0DE);

    let start = Instant::now();
    for f in 0..frames {
        update(&mut state, 0.016);

        // Rotate through tabs so every render path is exercised.
        state.tab = Tab::ALL[(f as usize / 3) % Tab::ALL.len()];
        state.table_scroll = (f as usize * 37) % state.procs.len();

        // Chaos: feed malformed/fragmented bytes into the input demuxer — must not panic.
        let payload: Vec<u8> = (0..32).map(|_| rng.u32() as u8).collect();
        for chunk in fuzzer.fuzz_payload(&payload) {
            let _ = demux.feed_bytes(&chunk);
        }
        // Fuzz the escape/path parsers too — hostile input must never panic mid-render.
        let s = String::from_utf8_lossy(&payload);
        let _ = SgrMouseParser::parse_sgr(&s);
        let _ = PathSanitizer::parse_drop_payload(&s);

        // Resize storm every 128 frames — boundary stability.
        if f % 128 == 0 && f > 0 {
            let nw = 40 + (rng.range(200)) as u16;
            let nh = 12 + (rng.range(50)) as u16;
            term.resize(nw, nh);
        }

        let _ = term.draw(|c| render(c, &mut state));
        state.dropped_frames = term_dropped(&term);
    }
    let elapsed = start.elapsed();
    println!(
        "done · {} frames in {:.2?} · {:.0} fps · 100k-row table · reactor reflows={} · no panics",
        frames,
        elapsed,
        frames as f64 / elapsed.as_secs_f64().max(1e-9),
        state.reactor.reflow_passes(),
    );
}

fn term_dropped<W: Write>(_t: &Terminal<W>) -> u64 {
    0
}

/// Perf-free timing breakdown (a poor-man's profiler for when `perf`/flamegraph isn't
/// available): times `update` and each tab's render in isolation, plus a full-screen SIMD
/// diff, then prints a sorted per-frame breakdown to point at hotspots to flamegraph.
fn run_profile(frames: u64) {
    println!("tenui showcase · profile mode · {} frames/section", frames);
    let (w, h) = (200u16, 60u16);
    let mut state = build_state(w, h);
    let mut buf = Buffer::new(w, h);
    let us = |d: Duration| d.as_secs_f64() * 1e6 / frames as f64;
    let mut rows: Vec<(String, f64)> = Vec::new();

    // update()
    let t = Instant::now();
    for _ in 0..frames {
        update(&mut state, 0.016);
    }
    rows.push(("update() [all subsystems]".into(), us(t.elapsed())));

    // Per-tab render.
    for tab in Tab::ALL {
        state.tab = tab;
        let t = Instant::now();
        for _ in 0..frames {
            let mut sv = buf.subview_mut(Rect::new(0, 0, w, h));
            render(&mut sv, &mut state);
        }
        rows.push((format!("render: {}", tab.title()), us(t.elapsed())));
    }

    // Full-screen SIMD differential scan (the retained-repaint diff path).
    let front = Buffer::new(w, h);
    let mut back = Buffer::new(w, h);
    back.set_string(5, 5, "hotspot churn", Color::Rgb(1, 2, 3), Color::Black, Modifier::BOLD);
    let t = Instant::now();
    let mut dirty = 0usize;
    for _ in 0..frames {
        dirty = SimdDiffScanner::diff_buffers(&front, &back).len();
    }
    rows.push((
        format!("simd diff_buffers ({}x{}, {} dirty)", w, h, dirty),
        us(t.elapsed()),
    ));

    rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    println!("\n  per-frame cost (µs), hottest first:");
    for (name, micros) in rows {
        println!("  {:>9.1} µs   {}", micros, name);
    }
    println!("\n  → flamegraph a hotspot precisely with:");
    println!(
        "     cargo flamegraph --example showcase -- --bench {}",
        frames.max(5000)
    );
}

impl State {
    fn status_tag(&self) -> String {
        format!("tab={}", self.tab.title())
    }
}

// A quit flag threaded through State (handle_key returns bool but the loop also checks this).
impl State {}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let frames_after = |flag: &str| -> Option<u64> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<u64>().ok())
    };
    a11y::set_reduce_motion(false);
    if args.iter().any(|a| a == "--bench") {
        run_bench(frames_after("--bench").unwrap_or(1000));
        Ok(())
    } else if args.iter().any(|a| a == "--profile") {
        run_profile(frames_after("--profile").unwrap_or(2000));
        Ok(())
    } else {
        run_interactive()
    }
}
