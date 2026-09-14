use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use taffy::{Point, prelude::*};
use tenui_core::{Buffer, CanvasSubviewMut, Color, InputEvent, Rect};
use tenui_ext::{
    middleware::{
        EventResult, InputMiddleware, KeymapTrieFilter, MacroRecorder, MiddlewareContext, MiddlewarePipeline,
        VimModalLayer, VimMode,
    },
    scripting::{ScriptEngineBridge, ScriptValue},
    widget::{CustomWidget, CustomWidgetContainer, Invalidation},
    wit::{CellPaint, LayoutConstraints, PluginHost, PluginPermissions, WidgetPlugin},
};

// 1. WASM Component Model WIT Plugin Tests

struct MockCounterPlugin {
    counter: u32,
}

impl WidgetPlugin for MockCounterPlugin {
    fn measure(&self, _avail_w: u16, _avail_h: u16) -> LayoutConstraints {
        LayoutConstraints {
            min_width: 10,
            max_width: 50,
            min_height: 1,
            max_height: 5,
        }
    }

    fn render(&self, _w: u16, _h: u16) -> Vec<CellPaint> {
        let text = format!("Count: {}", self.counter);
        text.chars()
            .enumerate()
            .map(|(i, c)| CellPaint::new(i as u16, 0, c.to_string(), 0x00FF00, 0x000000))
            .collect()
    }

    fn handle_key(&mut self, key_code: u32, _modifiers: u8) -> bool {
        if key_code == 43 {
            // '+'
            self.counter += 1;
            true
        } else {
            false
        }
    }

    fn save_state(&self) -> Vec<u8> {
        self.counter.to_le_bytes().to_vec()
    }

    fn load_state(&mut self, state: &[u8]) {
        if state.len() >= 4 {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&state[..4]);
            self.counter = u32::from_le_bytes(bytes);
        }
    }
}

#[test]
fn test_wit_plugin_lifecycle_and_hot_reload() {
    let initial_plugin = MockCounterPlugin { counter: 42 };
    let mut host = PluginHost::new("counter_plugin", initial_plugin, PluginPermissions::strict());

    // Paint initial frame
    let mut buf = Buffer::new(30, 5);
    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 30, 5));
        host.paint_to_canvas(&mut canvas);
    }
    let row0 = (0..15)
        .map(|x| buf.get(x, 0).map(|c| c.symbol.as_str()).unwrap_or(" "))
        .collect::<String>();
    assert!(row0.contains("Count: 42"));

    // Hot-reload with newly recompiled plugin instance
    let updated_plugin = MockCounterPlugin { counter: 0 };
    host.hot_reload(updated_plugin);

    // Assert zero-downtime state re-hydration (counter preserved from 42!)
    assert_eq!(host.plugin.counter, 42);
    assert_eq!(host.reload_count, 1);
}

// 2. CustomWidget & Two-Lane Invalidation Tests

struct MockCadViewer {
    dirty_paint: bool,
}

impl CustomWidget for MockCadViewer {
    fn query_invalidation(&self) -> Invalidation {
        if self.dirty_paint {
            Invalidation::PAINT
        } else {
            Invalidation::NONE
        }
    }

    fn layout_measure(
        &self,
        _known_dimensions: Size<Option<f32>>,
        _available_space: Size<AvailableSpace>,
    ) -> Size<f32> {
        Size {
            width: 20.0,
            height: 10.0,
        }
    }

    fn paint(&self, surface: &mut CanvasSubviewMut<'_>, _bounds: Layout) {
        surface.set_char(0, 0, 'C', Color::Cyan, Color::Reset, tenui_core::Modifier::empty());
        surface.set_char(1, 0, 'A', Color::Cyan, Color::Reset, tenui_core::Modifier::empty());
        surface.set_char(2, 0, 'D', Color::Cyan, Color::Reset, tenui_core::Modifier::empty());
    }

    fn hit_test(&self, local_x: u16, local_y: u16) -> bool {
        local_x < 3 && local_y == 0
    }
}

#[test]
fn test_custom_widget_two_lane_invalidation() {
    let cad = MockCadViewer { dirty_paint: true };
    let mut container = CustomWidgetContainer::new(cad);

    // Two-lane check: PaintOnly invalidation
    assert_eq!(container.poll_invalidation(), Invalidation::PAINT);

    let mut tree = TaffyTree::<()>::new();
    let node = tree
        .new_leaf(taffy::style::Style {
            size: Size {
                width: length(20.0f32),
                height: length(10.0f32),
            },
            ..Default::default()
        })
        .unwrap();
    tree.compute_layout(node, Size::MAX_CONTENT).unwrap();
    let mut layout = *tree.layout(node).unwrap();
    layout.location = Point { x: 5.0, y: 2.0 };

    let mut buf = Buffer::new(40, 15);
    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 40, 15));
        container.render(&mut canvas, layout);
    }

    // After render, invalidation is cleared to None
    assert_eq!(container.last_invalidation, Invalidation::NONE);

    // Hit test within widget bounds (5 + local_x)
    assert!(container.hit_test(5, 2)); // local (0, 0)
    assert!(container.hit_test(6, 2)); // local (1, 0)
    assert!(!container.hit_test(10, 2)); // local (5, 0) -> false
}

// 3. Tower Input Middleware Tests

#[test]
fn test_vim_modal_layer() {
    let mut vim = VimModalLayer::new();
    let mut cx = MiddlewareContext::default();

    assert_eq!(vim.mode, VimMode::Normal);
    assert_eq!(vim.mode_banner(), "-- NORMAL --");

    // Press 'i' -> switches to Insert
    let key_i = InputEvent::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    let res = vim.handle_event(&key_i, &mut cx, &mut |_ev, _ctx| EventResult::Ignored);
    assert_eq!(res, EventResult::Consumed);
    assert_eq!(vim.mode, VimMode::Insert);

    // In Insert mode, key passes through
    let key_a = InputEvent::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    let mut passed = false;
    vim.handle_event(&key_a, &mut cx, &mut |_ev, _ctx| {
        passed = true;
        EventResult::Consumed
    });
    assert!(passed);

    // Press Esc -> returns to Normal
    let key_esc = InputEvent::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    vim.handle_event(&key_esc, &mut cx, &mut |_ev, _ctx| EventResult::Ignored);
    assert_eq!(vim.mode, VimMode::Normal);

    // Press 'j' in normal mode -> motion down
    let key_j = InputEvent::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    vim.handle_event(&key_j, &mut cx, &mut |_ev, _ctx| EventResult::Ignored);
    assert_eq!(vim.last_motion, Some("down".into()));
}

#[test]
fn test_macro_recorder() {
    let mut recorder = MacroRecorder::new();
    let mut cx = MiddlewareContext::default();

    recorder.start_recording('q');
    assert!(recorder.is_recording);

    let ev1 = InputEvent::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    let ev2 = InputEvent::Key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));

    recorder.handle_event(&ev1, &mut cx, &mut |_ev, _ctx| EventResult::Ignored);
    recorder.handle_event(&ev2, &mut cx, &mut |_ev, _ctx| EventResult::Ignored);

    recorder.stop_recording();
    assert!(!recorder.is_recording);

    let macro_stream = recorder.get_macro('q').expect("Macro in register q");
    assert_eq!(macro_stream.len(), 2);
    assert_eq!(macro_stream[0], ev1);
    assert_eq!(macro_stream[1], ev2);
}

#[test]
fn test_keymap_trie_chords() {
    let mut trie = KeymapTrieFilter::new();
    let mut cx = MiddlewareContext::default();

    // Bind Ctrl+X Ctrl+F to write into context
    trie.bind(
        &[
            (KeyCode::Char('x'), KeyModifiers::CONTROL),
            (KeyCode::Char('f'), KeyModifiers::CONTROL),
        ],
        |ctx| {
            ctx.data.insert("action".into(), "find_file".into());
        },
    );

    let k1 = InputEvent::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));
    let k2 = InputEvent::Key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL));

    // First stroke -> buffered (Consumed)
    let res1 = trie.handle_event(&k1, &mut cx, &mut |_ev, _ctx| EventResult::Ignored);
    assert_eq!(res1, EventResult::Consumed);
    assert_eq!(cx.data.get("action"), None);

    // Second stroke -> triggers chord action
    let res2 = trie.handle_event(&k2, &mut cx, &mut |_ev, _ctx| EventResult::Ignored);
    assert_eq!(res2, EventResult::Consumed);
    assert_eq!(cx.data.get("action").map(|s| s.as_str()), Some("find_file"));
}

#[test]
fn test_middleware_pipeline_onion() {
    let mut pipeline = MiddlewarePipeline::new()
        .with_middleware(VimModalLayer::new())
        .with_middleware(MacroRecorder::new());

    let mut cx = MiddlewareContext::default();
    let ev = InputEvent::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));

    let res = pipeline.dispatch(&ev, &mut cx);
    assert_eq!(res, EventResult::Consumed);
}

// 4. Embedded Scripting Bridge Tests

#[test]
fn test_script_engine_bridge() {
    let mut bridge = ScriptEngineBridge::new();

    // 1. Command Registration
    bridge.register_command("add", |args| {
        let mut sum = 0;
        for arg in args {
            if let ScriptValue::Int(n) = arg {
                sum += n;
            }
        }
        Ok(ScriptValue::Int(sum))
    });

    let res = bridge
        .execute_command("add", &[ScriptValue::Int(10), ScriptValue::Int(25)])
        .unwrap();
    assert_eq!(res, ScriptValue::Int(35));

    // 2. Reactive Signals
    bridge.create_signal("counter", ScriptValue::Int(0));
    assert_eq!(bridge.get_signal("counter"), Some(ScriptValue::Int(0)));

    let observed = Vec::new();
    let obs_ref = std::sync::Arc::new(std::sync::Mutex::new(observed));
    let obs_clone = obs_ref.clone();

    bridge
        .listen_signal("counter", move |val| {
            if let ScriptValue::Int(n) = val {
                obs_clone.lock().unwrap().push(*n);
            }
        })
        .unwrap();

    bridge
        .update_signal("counter", |val| {
            if let ScriptValue::Int(n) = val {
                ScriptValue::Int(n + 1)
            } else {
                ScriptValue::Int(1)
            }
        })
        .unwrap();

    assert_eq!(bridge.get_signal("counter"), Some(ScriptValue::Int(1)));
    assert_eq!(*obs_ref.lock().unwrap(), vec![1]);
}
