use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tenui_core::{InputEvent, clipboard::ClipboardCommand};

/// Outcome of processing an event in the middleware pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EventResult {
    #[default]
    Ignored,
    Consumed,
}

/// Execution context passed along the middleware chain.
#[derive(Debug, Clone, Default)]
pub struct MiddlewareContext {
    pub focused_target: Option<u64>,
    /// Text carried alongside a paste (from a bracketed-paste event).
    pub clipboard: Option<String>,
    /// A clipboard action recognized this dispatch, for the app to apply to the focused
    /// [`ClipboardTarget`](tenui_core::ClipboardTarget) via
    /// [`apply_clipboard_command`](tenui_core::apply_clipboard_command).
    pub clipboard_command: Option<ClipboardCommand>,
    pub data: HashMap<String, String>,
}

/// Composable event interceptor inspired by Tower.
pub trait InputMiddleware: Send + Sync {
    fn handle_event(
        &mut self,
        event: &InputEvent,
        cx: &mut MiddlewareContext,
        next: &mut dyn FnMut(&InputEvent, &mut MiddlewareContext) -> EventResult,
    ) -> EventResult;
}

/// Middleware pipeline dispatching input events across a chain of interceptors.
#[derive(Default)]
pub struct MiddlewarePipeline {
    middlewares: Vec<Box<dyn InputMiddleware>>,
}

impl MiddlewarePipeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_middleware<M: InputMiddleware + 'static>(mut self, mw: M) -> Self {
        self.middlewares.push(Box::new(mw));
        self
    }

    pub fn push<M: InputMiddleware + 'static>(&mut self, mw: M) {
        self.middlewares.push(Box::new(mw));
    }

    /// Dispatches an event through the middleware onion chain.
    pub fn dispatch(&mut self, event: &InputEvent, cx: &mut MiddlewareContext) -> EventResult {
        Self::dispatch_step(&mut self.middlewares, 0, event, cx)
    }

    fn dispatch_step(
        middlewares: &mut [Box<dyn InputMiddleware>],
        index: usize,
        event: &InputEvent,
        cx: &mut MiddlewareContext,
    ) -> EventResult {
        if index >= middlewares.len() {
            return EventResult::Ignored;
        }

        let (head, tail) = middlewares[index..].split_at_mut(1);
        let mw = &mut head[0];

        mw.handle_event(event, cx, &mut |ev, ctx| Self::dispatch_step(tail, 0, ev, ctx))
    }
}

/// Middleware that maps clipboard key chords (Ctrl+C/X/V/A) and bracketed-paste events to
/// a [`ClipboardCommand`] on the context, consuming the event. It stays widget-agnostic —
/// after `dispatch`, the app applies `cx.clipboard_command` to its focused
/// [`ClipboardTarget`](tenui_core::ClipboardTarget):
///
/// ```ignore
/// let r = pipeline.dispatch(&event, &mut cx);
/// if let Some(cmd) = cx.clipboard_command.take() {
///     if let Some(text) = cx.clipboard.take() { clipboard.set_register(text); } // bracketed paste
///     if let Some(esc) = apply_clipboard_command(&mut focused_widget, cmd, &mut clipboard) {
///         terminal.emit(&esc)?; // push OSC 52 to the system clipboard
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct ClipboardLayer;

impl ClipboardLayer {
    pub fn new() -> Self {
        Self
    }
}

impl InputMiddleware for ClipboardLayer {
    fn handle_event(
        &mut self,
        event: &InputEvent,
        cx: &mut MiddlewareContext,
        next: &mut dyn FnMut(&InputEvent, &mut MiddlewareContext) -> EventResult,
    ) -> EventResult {
        match event {
            InputEvent::Key(key) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let cmd = match key.code {
                    KeyCode::Char('c') | KeyCode::Char('C') => Some(ClipboardCommand::Copy),
                    KeyCode::Char('x') | KeyCode::Char('X') => Some(ClipboardCommand::Cut),
                    KeyCode::Char('v') | KeyCode::Char('V') => Some(ClipboardCommand::Paste),
                    KeyCode::Char('a') | KeyCode::Char('A') => Some(ClipboardCommand::SelectAll),
                    _ => None,
                };
                if let Some(cmd) = cmd {
                    cx.clipboard_command = Some(cmd);
                    return EventResult::Consumed;
                }
            }
            // Bracketed paste: carry the text and request a paste.
            InputEvent::Paste(text) => {
                cx.clipboard = Some(text.clone());
                cx.clipboard_command = Some(ClipboardCommand::Paste);
                return EventResult::Consumed;
            }
            _ => {}
        }
        next(event, cx)
    }
}

/// Modal states supported by the Vim emulation layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VimMode {
    #[default]
    Normal,
    Insert,
    Visual,
    Command,
}

/// Vim modal navigation and editing middleware layer.
#[derive(Debug, Clone)]
pub struct VimModalLayer {
    pub mode: VimMode,
    pub command_buffer: String,
    pub last_motion: Option<String>,
}

impl Default for VimModalLayer {
    fn default() -> Self {
        Self {
            mode: VimMode::Normal,
            command_buffer: String::new(),
            last_motion: None,
        }
    }
}

impl VimModalLayer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mode_banner(&self) -> &'static str {
        match self.mode {
            VimMode::Normal => "-- NORMAL --",
            VimMode::Insert => "-- INSERT --",
            VimMode::Visual => "-- VISUAL --",
            VimMode::Command => "-- COMMAND --",
        }
    }
}

impl InputMiddleware for VimModalLayer {
    fn handle_event(
        &mut self,
        event: &InputEvent,
        cx: &mut MiddlewareContext,
        next: &mut dyn FnMut(&InputEvent, &mut MiddlewareContext) -> EventResult,
    ) -> EventResult {
        if let InputEvent::Key(key) = event {
            // Global Esc returns to Normal mode
            if key.code == KeyCode::Esc {
                self.mode = VimMode::Normal;
                self.command_buffer.clear();
                return EventResult::Consumed;
            }

            match self.mode {
                VimMode::Insert => {
                    // In insert mode, pass keys through to downstream editor
                    return next(event, cx);
                }
                VimMode::Normal => {
                    match key.code {
                        KeyCode::Char('i') => {
                            self.mode = VimMode::Insert;
                            return EventResult::Consumed;
                        }
                        KeyCode::Char('v') => {
                            self.mode = VimMode::Visual;
                            return EventResult::Consumed;
                        }
                        KeyCode::Char(':') => {
                            self.mode = VimMode::Command;
                            self.command_buffer.clear();
                            return EventResult::Consumed;
                        }
                        // Standard Vim Motions
                        KeyCode::Char('h') => {
                            self.last_motion = Some("left".into());
                            return EventResult::Consumed;
                        }
                        KeyCode::Char('j') => {
                            self.last_motion = Some("down".into());
                            return EventResult::Consumed;
                        }
                        KeyCode::Char('k') => {
                            self.last_motion = Some("up".into());
                            return EventResult::Consumed;
                        }
                        KeyCode::Char('l') => {
                            self.last_motion = Some("right".into());
                            return EventResult::Consumed;
                        }
                        KeyCode::Char('w') => {
                            self.last_motion = Some("word_forward".into());
                            return EventResult::Consumed;
                        }
                        KeyCode::Char('b') => {
                            self.last_motion = Some("word_backward".into());
                            return EventResult::Consumed;
                        }
                        _ => {}
                    }
                }
                VimMode::Visual => {
                    match key.code {
                        KeyCode::Char('y') => {
                            // Yank selection
                            self.mode = VimMode::Normal;
                            return EventResult::Consumed;
                        }
                        KeyCode::Char('d') => {
                            // Delete selection
                            self.mode = VimMode::Normal;
                            return EventResult::Consumed;
                        }
                        _ => {}
                    }
                }
                VimMode::Command => {
                    match key.code {
                        KeyCode::Enter => {
                            // Execute command buffer
                            cx.data.insert("vim_command".into(), self.command_buffer.clone());
                            self.command_buffer.clear();
                            self.mode = VimMode::Normal;
                            return EventResult::Consumed;
                        }
                        KeyCode::Backspace => {
                            self.command_buffer.pop();
                            return EventResult::Consumed;
                        }
                        KeyCode::Char(c) => {
                            self.command_buffer.push(c);
                            return EventResult::Consumed;
                        }
                        _ => {}
                    }
                }
            }
        }

        next(event, cx)
    }
}

/// Macro recording middleware capturing deterministic event streams into named registers.
#[derive(Debug, Clone, Default)]
pub struct MacroRecorder {
    pub registers: HashMap<char, Vec<InputEvent>>,
    pub current_register: Option<char>,
    pub is_recording: bool,
}

impl MacroRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_recording(&mut self, register: char) {
        self.current_register = Some(register);
        self.registers.insert(register, Vec::new());
        self.is_recording = true;
    }

    pub fn stop_recording(&mut self) {
        self.is_recording = false;
        self.current_register = None;
    }

    pub fn get_macro(&self, register: char) -> Option<&[InputEvent]> {
        self.registers.get(&register).map(|v| v.as_slice())
    }
}

impl InputMiddleware for MacroRecorder {
    fn handle_event(
        &mut self,
        event: &InputEvent,
        cx: &mut MiddlewareContext,
        next: &mut dyn FnMut(&InputEvent, &mut MiddlewareContext) -> EventResult,
    ) -> EventResult {
        if self.is_recording
            && let Some(reg) = self.current_register
            && let Some(queue) = self.registers.get_mut(&reg)
        {
            queue.push(event.clone());
        }
        next(event, cx)
    }
}

pub type KeymapAction = Box<dyn Fn(&mut MiddlewareContext) + Send + Sync>;

/// Multi-stroke key chord router (e.g. `Ctrl+X Ctrl+F` or `Space f f`).
#[derive(Default)]
pub struct KeymapTrieFilter {
    active_sequence: Vec<(KeyCode, KeyModifiers)>,
    routes: HashMap<Vec<(KeyCode, KeyModifiers)>, KeymapAction>,
}

impl KeymapTrieFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind<F>(&mut self, chord: &[(KeyCode, KeyModifiers)], action: F)
    where
        F: Fn(&mut MiddlewareContext) + Send + Sync + 'static,
    {
        self.routes.insert(chord.to_vec(), Box::new(action));
    }

    pub fn reset_chord(&mut self) {
        self.active_sequence.clear();
    }
}

impl InputMiddleware for KeymapTrieFilter {
    fn handle_event(
        &mut self,
        event: &InputEvent,
        cx: &mut MiddlewareContext,
        next: &mut dyn FnMut(&InputEvent, &mut MiddlewareContext) -> EventResult,
    ) -> EventResult {
        if let InputEvent::Key(KeyEvent { code, modifiers, .. }) = event {
            self.active_sequence.push((*code, *modifiers));

            // Check if exact match exists
            if let Some(action) = self.routes.get(&self.active_sequence) {
                action(cx);
                self.active_sequence.clear();
                return EventResult::Consumed;
            }

            // Check if any route starts with current prefix
            let is_prefix = self.routes.keys().any(|k| k.starts_with(&self.active_sequence));
            if is_prefix {
                // Buffer chord and wait for next stroke
                return EventResult::Consumed;
            }

            // Not a prefix: reset chord and pass through
            self.active_sequence.clear();
        }

        next(event, cx)
    }
}

#[cfg(test)]
mod clipboard_layer_tests {
    use super::*;

    fn ctrl(c: char) -> InputEvent {
        InputEvent::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL))
    }

    #[test]
    fn test_ctrl_chords_map_to_commands() {
        let mut pipe = MiddlewarePipeline::new().with_middleware(ClipboardLayer::new());

        let mut cx = MiddlewareContext::default();
        assert_eq!(pipe.dispatch(&ctrl('c'), &mut cx), EventResult::Consumed);
        assert_eq!(cx.clipboard_command, Some(ClipboardCommand::Copy));

        let mut cx = MiddlewareContext::default();
        pipe.dispatch(&ctrl('x'), &mut cx);
        assert_eq!(cx.clipboard_command, Some(ClipboardCommand::Cut));

        let mut cx = MiddlewareContext::default();
        pipe.dispatch(&ctrl('v'), &mut cx);
        assert_eq!(cx.clipboard_command, Some(ClipboardCommand::Paste));

        let mut cx = MiddlewareContext::default();
        pipe.dispatch(&ctrl('a'), &mut cx);
        assert_eq!(cx.clipboard_command, Some(ClipboardCommand::SelectAll));
    }

    #[test]
    fn test_bracketed_paste_carries_text() {
        let mut pipe = MiddlewarePipeline::new().with_middleware(ClipboardLayer::new());
        let mut cx = MiddlewareContext::default();
        let r = pipe.dispatch(&InputEvent::Paste("hi there".into()), &mut cx);
        assert_eq!(r, EventResult::Consumed);
        assert_eq!(cx.clipboard_command, Some(ClipboardCommand::Paste));
        assert_eq!(cx.clipboard.as_deref(), Some("hi there"));
    }

    #[test]
    fn test_non_clipboard_key_passes_through() {
        let mut pipe = MiddlewarePipeline::new().with_middleware(ClipboardLayer::new());
        let mut cx = MiddlewareContext::default();
        // Plain 'c' (no CONTROL) is not a clipboard chord.
        let ev = InputEvent::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        let r = pipe.dispatch(&ev, &mut cx);
        assert_eq!(r, EventResult::Ignored); // no downstream consumed it
        assert_eq!(cx.clipboard_command, None);
    }
}
