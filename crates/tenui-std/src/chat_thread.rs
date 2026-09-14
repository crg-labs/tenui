use tenui_core::{CanvasSubviewMut, Clipboard, Color};

use crate::selection::DocumentSelection;

/// Role for conversational messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// Collapsible interactive tool-call card.
#[derive(Clone, Debug)]
pub struct ToolCallCard {
    pub call_id: Option<String>,
    pub tool_name: String,
    pub parameters_json: String,
    pub output: Option<String>,
    pub is_collapsed: bool,
}

impl ToolCallCard {
    pub fn new(tool_name: impl Into<String>, parameters_json: impl Into<String>) -> Self {
        Self {
            call_id: None,
            tool_name: tool_name.into(),
            parameters_json: parameters_json.into(),
            output: None,
            is_collapsed: true,
        }
    }

    pub fn with_call_id(mut self, call_id: impl Into<String>) -> Self {
        self.call_id = Some(call_id.into());
        self
    }

    pub fn toggle_collapsed(&mut self) {
        self.is_collapsed = !self.is_collapsed;
    }
}

/// Collapsible reasoning ("thinking") trace emitted by a model before its answer.
///
/// Modern reasoning models (e.g. DeepSeek-R1, QwQ, o-series) produce a chain of
/// thought that is distinct from both the visible answer ([`ChatMessage::content`])
/// and any tool calls. It arrives on its own stream channel — `reasoning`/
/// `reasoning_content` in OpenAI-compatible deltas, or inline `<think>…</think>`
/// spans — so the thread models it separately and renders it as a foldable card,
/// collapsed by default (secondary to the answer) with a live indicator while the
/// model is still reasoning.
#[derive(Clone, Debug)]
pub struct ThinkingBlock {
    pub content: String,
    pub is_collapsed: bool,
    /// `true` while reasoning tokens are still streaming in.
    pub is_active: bool,
}

impl ThinkingBlock {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            is_collapsed: true,
            is_active: true,
        }
    }

    pub fn toggle_collapsed(&mut self) {
        self.is_collapsed = !self.is_collapsed;
    }

    pub fn append_token(&mut self, token: &str) {
        self.content.push_str(token);
    }
}

impl Default for ThinkingBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// A single message in the conversational thread.
#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    /// Reasoning trace that preceded [`content`](Self::content), when the model emits one.
    pub thinking: Option<ThinkingBlock>,
    pub tool_call: Option<ToolCallCard>,
    pub is_streaming: bool,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            thinking: None,
            tool_call: None,
            is_streaming: false,
        }
    }

    pub fn assistant_streaming() -> Self {
        Self {
            role: MessageRole::Assistant,
            content: String::new(),
            thinking: None,
            tool_call: None,
            is_streaming: true,
        }
    }

    pub fn with_tool_call(mut self, card: ToolCallCard) -> Self {
        self.tool_call = Some(card);
        self
    }

    /// Attaches a reasoning trace (builder-style), e.g. for a pre-recorded transcript.
    pub fn with_thinking(mut self, block: ThinkingBlock) -> Self {
        self.thinking = Some(block);
        self
    }

    pub fn append_token(&mut self, token: &str) {
        self.content.push_str(token);
    }

    /// Appends a reasoning token, lazily creating an active [`ThinkingBlock`] on first use.
    pub fn append_thinking_token(&mut self, token: &str) {
        self.thinking.get_or_insert_with(ThinkingBlock::new).append_token(token);
    }
}

/// Conversational thread surface supporting incremental token streaming and tool cards.
pub struct ChatThread {
    pub messages: Vec<ChatMessage>,
    pub auto_tail_lock: bool,
    pub scroll_offset: usize,
    /// Click-and-drag text selection over the rendered transcript.
    pub selection: DocumentSelection,
    /// Plain `(x_offset, text)` of each rendered visual row, captured on the last
    /// [`render`](Self::render) so selection can extract and highlight it.
    rendered_rows: Vec<(u16, String)>,
}

impl ChatThread {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            auto_tail_lock: true,
            scroll_offset: 0,
            selection: DocumentSelection::new(),
            rendered_rows: Vec::new(),
        }
    }

    /// Begins a selection drag at rendered cell (`x`, `y`) (mouse press).
    pub fn on_mouse_down(&mut self, x: u16, y: u16) {
        self.selection.begin(y as usize, x);
    }

    /// Extends the in-progress selection to cell (`x`, `y`) (mouse move while pressed).
    pub fn on_mouse_drag(&mut self, x: u16, y: u16) {
        self.selection.extend(y as usize, x);
    }

    /// Ends the selection drag (mouse release).
    pub fn on_mouse_up(&mut self) {
        self.selection.finish();
    }

    /// Clears any active transcript selection.
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    /// The transcript text currently selected (empty when nothing is selected), reflecting the
    /// layout of the most recent [`render`](Self::render). Copy it with a [`Clipboard`].
    pub fn selected_text(&self) -> String {
        self.selection.extract(&self.rendered_rows)
    }

    pub fn add_message(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
        if self.auto_tail_lock {
            self.scroll_to_bottom();
        }
    }

    pub fn start_streaming_assistant(&mut self) {
        self.add_message(ChatMessage::assistant_streaming());
    }

    pub fn finish_streaming(&mut self) {
        if let Some(last) = self.messages.last_mut() {
            last.is_streaming = false;
        }
    }

    pub fn append_streaming_token(&mut self, token: &str) {
        if let Some(last) = self.messages.last_mut()
            && last.is_streaming
        {
            last.append_token(token);
        }
    }

    /// Appends a reasoning token to the in-flight assistant message, creating its
    /// [`ThinkingBlock`] on first use. Pair with [`finish_thinking`](Self::finish_thinking)
    /// when the model transitions from reasoning to its visible answer.
    pub fn append_streaming_thinking(&mut self, token: &str) {
        if let Some(last) = self.messages.last_mut()
            && last.is_streaming
        {
            last.append_thinking_token(token);
        }
    }

    /// Marks the in-flight message's reasoning trace complete (the model has begun its
    /// answer). Leaves the trace itself intact and collapsed.
    pub fn finish_thinking(&mut self) {
        if let Some(last) = self.messages.last_mut()
            && let Some(thinking) = &mut last.thinking
        {
            thinking.is_active = false;
        }
    }

    pub fn toggle_tool_card(&mut self, message_idx: usize) {
        if let Some(msg) = self.messages.get_mut(message_idx)
            && let Some(tool) = &mut msg.tool_call
        {
            tool.toggle_collapsed();
        }
    }

    /// Folds/unfolds the reasoning card on message `message_idx`.
    pub fn toggle_thinking_card(&mut self, message_idx: usize) {
        if let Some(msg) = self.messages.get_mut(message_idx)
            && let Some(thinking) = &mut msg.thinking
        {
            thinking.toggle_collapsed();
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.messages.len().saturating_sub(1);
    }

    /// Plain-text rendering of one message for clipboard/export (no UI glyphs).
    fn message_plain_text(msg: &ChatMessage) -> String {
        let role = match msg.role {
            MessageRole::User => "You",
            MessageRole::Assistant => "Assistant",
            MessageRole::System => "System",
            MessageRole::Tool => "Tool",
        };
        let mut out = format!("{}: {}", role, msg.content);
        if let Some(thinking) = &msg.thinking
            && !thinking.content.is_empty()
        {
            out.push_str("\n[Thinking]\n");
            out.push_str(&thinking.content);
        }
        if let Some(tool) = &msg.tool_call {
            out.push_str(&format!("\n[Tool Call: {}] {}", tool.tool_name, tool.parameters_json));
            if let Some(output) = &tool.output {
                out.push('\n');
                out.push_str(output);
            }
        }
        out
    }

    /// Copies message `idx` into `clip` and returns the OSC 52 escape to emit to the
    /// terminal (or `None` when `idx` is out of range).
    pub fn copy_message(&self, idx: usize, clip: &mut Clipboard) -> Option<String> {
        let msg = self.messages.get(idx)?;
        Some(clip.copy(&Self::message_plain_text(msg)))
    }

    /// Copies the whole transcript into `clip` and returns the OSC 52 escape to emit.
    pub fn copy_all(&self, clip: &mut Clipboard) -> String {
        let transcript = self
            .messages
            .iter()
            .map(Self::message_plain_text)
            .collect::<Vec<_>>()
            .join("\n\n");
        clip.copy(&transcript)
    }

    /// Renders chat messages and foldable tool cards into the canvas subview, capturing each
    /// visual row for text selection and painting any active selection highlight.
    pub fn render(&mut self, surface: &mut CanvasSubviewMut<'_>, default_bg: Color) {
        let w = surface.width();
        let h = surface.height();
        self.rendered_rows.clear();
        if w < 10 || h == 0 || self.messages.is_empty() {
            return;
        }

        // Each drawn line is recorded as (x_offset, text) at its screen row so the cache index
        // matches the row coordinate the selection uses (this widget renders from the top, so
        // screen y == document row).
        let mut rows: Vec<(u16, String)> = Vec::new();
        let mut curr_y = 0u16;

        for msg in self.messages.iter() {
            if curr_y >= h {
                break;
            }

            // Role Banner
            let (role_tag, role_color) = match msg.role {
                MessageRole::User => ("▶ You:", Color::CYAN),
                MessageRole::Assistant => ("🤖 Assistant:", Color::GREEN),
                MessageRole::System => ("⚙ System:", Color::YELLOW),
                MessageRole::Tool => ("🔧 Tool:", Color::LightMagenta),
            };

            surface.write_str_clipped(1, curr_y, role_tag, role_color, default_bg);
            rows.push((1, role_tag.to_string()));
            curr_y += 1;
            if curr_y >= h {
                break;
            }

            // Foldable Reasoning ("thinking") card — precedes the answer, collapsed by
            // default so it never buries the visible content. A live "…" marker shows while
            // the model is still reasoning; expanded, it prints the dim reasoning lines.
            if let Some(thinking) = &msg.thinking
                && curr_y < h
            {
                let fold_icon = if thinking.is_collapsed { "▶" } else { "▼" };
                let label = if thinking.is_active { "Thinking…" } else { "Thinking" };
                let header = format!("  {} 💭 {}", fold_icon, label);
                surface.write_str_clipped(2, curr_y, &header, Color::DarkGray, default_bg);
                rows.push((2, header));
                curr_y += 1;

                if !thinking.is_collapsed {
                    for line in thinking.content.lines() {
                        if curr_y >= h {
                            break;
                        }
                        surface.write_str_clipped(4, curr_y, line, Color::GRAY, default_bg);
                        rows.push((4, line.to_string()));
                        curr_y += 1;
                    }
                }
            }

            // Message Body
            for line in msg.content.lines() {
                if curr_y >= h {
                    break;
                }
                surface.write_str_clipped(3, curr_y, line, Color::WHITE, default_bg);
                rows.push((3, line.to_string()));
                curr_y += 1;
            }

            // Streaming Cursor (overlay on the previous row; not a selectable row of its own)
            if msg.is_streaming && curr_y > 0 && curr_y <= h {
                surface.write_str_clipped(3 + msg.content.len() as u16, curr_y - 1, " ▋", Color::CYAN, default_bg);
            }

            // Foldable Tool Card
            if let Some(tool) = &msg.tool_call
                && curr_y < h
            {
                let fold_icon = if tool.is_collapsed { "▶" } else { "▼" };
                let header = format!("  {} [Tool Call: {}]", fold_icon, tool.tool_name);
                surface.write_str_clipped(2, curr_y, &header, Color::YELLOW, Color::Rgb(30, 41, 59));
                rows.push((2, header));
                curr_y += 1;

                if !tool.is_collapsed && curr_y < h {
                    surface.write_str_clipped(4, curr_y, &tool.parameters_json, Color::GRAY, default_bg);
                    rows.push((4, tool.parameters_json.clone()));
                    curr_y += 1;
                }
            }

            // Spacer line (blank, but still a row for selection alignment)
            rows.push((0, String::new()));
            curr_y += 1;
        }

        self.rendered_rows = rows;
        self.selection
            .highlight(surface, 0, &self.rendered_rows, Color::CYAN, Color::Black);
    }
}

impl Default for ChatThread {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_message() {
        let mut thread = ChatThread::new();
        thread.add_message(ChatMessage::user("hello there"));
        let mut clip = Clipboard::new();
        let esc = thread.copy_message(0, &mut clip).expect("message exists");
        assert_eq!(clip.register(), "You: hello there");
        assert!(esc.starts_with("\x1b]52;c;"));
        assert!(thread.copy_message(5, &mut clip).is_none());
    }

    #[test]
    fn test_copy_all_transcript() {
        let mut thread = ChatThread::new();
        thread.add_message(ChatMessage::user("hi"));
        let mut a = ChatMessage::assistant_streaming();
        a.append_token("there");
        a.is_streaming = false;
        thread.add_message(a);

        let mut clip = Clipboard::new();
        thread.copy_all(&mut clip);
        assert_eq!(clip.register(), "You: hi\n\nAssistant: there");
    }

    #[test]
    fn test_streaming_thinking_then_answer() {
        let mut thread = ChatThread::new();
        thread.add_message(ChatMessage::user("why is the sky blue?"));
        thread.start_streaming_assistant();

        // Reasoning streams first, then the visible answer.
        thread.append_streaming_thinking("Rayleigh ");
        thread.append_streaming_thinking("scattering.");
        thread.finish_thinking();
        thread.append_streaming_token("Because of Rayleigh scattering.");
        thread.finish_streaming();

        let msg = thread.messages.last().expect("assistant message");
        let thinking = msg.thinking.as_ref().expect("thinking block");
        assert_eq!(thinking.content, "Rayleigh scattering.");
        assert!(!thinking.is_active, "thinking marked complete");
        assert!(thinking.is_collapsed, "reasoning collapsed by default");
        assert_eq!(msg.content, "Because of Rayleigh scattering.");
    }

    #[test]
    fn test_toggle_thinking_card() {
        let mut thread = ChatThread::new();
        thread.start_streaming_assistant();
        thread.append_streaming_thinking("step 1");
        assert!(thread.messages[0].thinking.as_ref().unwrap().is_collapsed);
        thread.toggle_thinking_card(0);
        assert!(!thread.messages[0].thinking.as_ref().unwrap().is_collapsed);
    }

    #[test]
    fn test_copy_includes_thinking() {
        let mut thread = ChatThread::new();
        let msg = ChatMessage::user("go").with_thinking({
            let mut t = ThinkingBlock::new();
            t.append_token("pondering");
            t
        });
        thread.add_message(msg);
        let mut clip = Clipboard::new();
        thread.copy_message(0, &mut clip);
        assert_eq!(clip.register(), "You: go\n[Thinking]\npondering");
    }

    #[test]
    fn test_thread_text_selection_and_copy() {
        use tenui_core::{Buffer, Rect};
        let mut thread = ChatThread::new();
        thread.add_message(ChatMessage::user("hello there"));

        let mut buf = Buffer::new(40, 10);
        {
            let mut sv = buf.subview_mut(Rect::new(0, 0, 40, 10));
            thread.render(&mut sv, Color::Reset);
        }
        // Body "hello there" renders at row y=1, starting at column 3. Select "hello".
        thread.on_mouse_down(3, 1);
        thread.on_mouse_drag(7, 1);
        thread.on_mouse_up();
        assert_eq!(thread.selected_text(), "hello");

        // The highlight inverted those cells to the selection background.
        assert_eq!(buf.get(3, 1).unwrap().symbol.as_str(), "h");

        thread.clear_selection();
        assert!(thread.selected_text().is_empty());
    }

    #[test]
    fn test_copy_message_includes_tool_call() {
        let mut thread = ChatThread::new();
        let mut card = ToolCallCard::new("search", "{\"q\":\"rust\"}");
        card.output = Some("42 results".into());
        thread.add_message(ChatMessage::user("go").with_tool_call(card));
        let mut clip = Clipboard::new();
        thread.copy_message(0, &mut clip);
        assert_eq!(
            clip.register(),
            "You: go\n[Tool Call: search] {\"q\":\"rust\"}\n42 results"
        );
    }
}
