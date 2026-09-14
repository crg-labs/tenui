# Tenui Standard Widgets: A Case Study with the Chat Application

The [`tenui-std`](https://docs.rs/tenui-std) crate contains production-ready TUI widgets. To demonstrate how these components interact in a real application, this guide walks through the architecture of the **tenui-chat** client (a standalone project built on this framework).

---

## 1. The Three-Zone Shell

The chat application employs a responsive three-zone shell:

```text
┌─────────────────────────┬────────────────────────────────────────────────────┐
│                         │ Header (Active Model, Session Title, Settings Icon)│
│                         ├────────────────────────────────────────────────────┤
│                         │                                                    │
│    History Sidebar      │                                                    │
│                         │            Conversation Transcript                 │
│  • Local persistence    │                                                    │
│  • Resizable mouse drag │  • Collapsible thinking cards                      │
│  • Context action popup │  • Tool call cards                                 │
│  • Search & filter      │  • Streaming token rendering                       │
│                         │  • Code blocks with syntax highlighting & copy     │
│                         │  • Formatted Markdown tables                       │
│                         │                                                    │
│                         ├────────────────────────────────────────────────────┤
│                         │ Composer & Attachment Staging Area                 │
│                         │ • Multiline editing with cursor navigation         │
│                         │ • Selection range & clipboard copy/cut/paste       │
│                         │ • Automatic large paste interception               │
└─────────────────────────┴────────────────────────────────────────────────────┘
```

The outer shell is constructed using `tenui-layout` flexbox primitives, dividing screen space between the sidebar and the active conversation:

```rust,no_run
terminal.draw_layout(|ui| {
    ui.row(Style::default().flex_row(), |ui| {
        // Zone 1: Sidebar (dynamically resizable width)
        ui.leaf(Style::default().width_cells(sidebar_width as f32), |subview| {
            render_sidebar(subview, &app_state);
        });

        // Zone 2 & 3: Main column (Transcript + Composer)
        ui.column(Style::default().grow(1.0), |ui| {
            ui.leaf(Style::default().height_cells(3.0), |subview| {
                render_header(subview, &app_state);
            });
            ui.leaf(Style::default().grow(1.0), |subview| {
                render_transcript(subview, &app_state);
            });
            ui.leaf(Style::default().height_cells(composer_height as f32), |subview| {
                render_composer(subview, &app_state);
            });
        });
    });
})?;
```

---

## 2. Design System & Theming (`ThemePalette`)

Consistency across dark and light terminal environments is governed by [`ThemePalette`](https://docs.rs/tenui-std/latest/tenui_std/struct.ThemePalette.html). Rather than hardcoding terminal colors, widgets consume semantic design tokens:

- `palette.bg_surface`: Main background fill.
- `palette.bg_panel`: Elevated panel or container background.
- `palette.accent`: Primary focus and selection indicator.
- `palette.text_primary`: High-contrast foreground text.
- `palette.text_muted`: Secondary hints and metadata labels.

### WCAG 2.1 Contrast Auditing

Tenui automatically audits color pairings using [`audit_contrast`](https://docs.rs/tenui-std/latest/tenui_std/fn.audit_contrast.html). If a user's terminal configuration results in low-contrast text (contrast ratio < 4.5:1), [`ensure_contrast`](https://docs.rs/tenui-std/latest/tenui_std/fn.ensure_contrast.html) shifts the foreground color along the CIELAB $L^*$ lightness axis until WCAG AA compliance is achieved.

---

## 3. Streaming Transcript & Reasoning Cards (`ChatThread`)

The conversation feed is managed by [`ChatThread`](https://docs.rs/tenui-std/latest/tenui_std/struct.ChatThread.html), supporting streaming tokens from modern LLMs:

### Collapsible Thinking Blocks ([`ThinkingBlock`])
Reasoning models (e.g. DeepSeek-R1, QwQ, o-series) output chain-of-thought traces before the final answer. `ThinkingBlock` encapsulates this trace:
- **Collapsed View**: Renders a compact summary badge: `╭─ Thought (3.4s) [▸ Expand] ─────────────╮`.
- **Expanded View**: Reveals the step-by-step reasoning steps.
- **Mouse & Keyboard Toggle**: Clickable via mouse or toggled with keyboard shortcuts.

### Tool Call Cards ([`ToolCallCard`])
When the assistant invokes external functions, `ToolCallCard` renders an interactive block showing:
- Tool function name (e.g. `read_file`, `search_web`).
- JSON input arguments.
- Returned execution output with status indicators.

### Syntax-Highlighted Code Viewers ([`CodeView`])
Code snippets within the assistant response are tokenized via `CodeView`:
- Language badge displayed in header (`rust`, `python`, `json`).
- `[📋 Copy]` button bound to mouse clicks and OSC 52 clipboard export.
- Indented continuation markers (`  ↳ `) for lines wrapping past viewport width.

---

## 4. Rich Composer & Attachment Area (`TextInput`)

The input area at the bottom of the interface handles multiline composition:

- **Cursor Navigation & Selection**: Supports <kbd>Left</kbd>, <kbd>Right</kbd>, <kbd>Home</kbd>, <kbd>End</kbd>, and column-aligned <kbd>Up</kbd> / <kbd>Down</kbd> line wrapping navigation.
- **Selection Ranges**: <kbd>Shift+Arrows</kbd> anchors and extends text selection ranges, rendered with inverse color highlighting.
- **OS Clipboard Integration**: Seamless <kbd>Ctrl+C</kbd>, <kbd>Ctrl+X</kbd>, and <kbd>Ctrl+V</kbd> operations backed by `tenui_core::Clipboard`.
- **Large Paste Interception**: Pasting huge payloads (>500 lines) freezes traditional terminals. The composer intercepts large pastes and automatically packages them as staged snippet attachments, keeping the interactive input responsive.

---

## 5. History Sidebar & Persistence

The sidebar displays conversation history with local persistence:

- **Dynamic Mouse Resizing**: Hovering the border between sidebar and transcript shows a drag affordance. Dragging adjusts the sidebar width in real time.
- **Context Action Menu**: Right-clicking a session opens a contextual popup ([`Popover`]) with actions:
  - **Export Session** (to Markdown or JSON)
  - **Rename Conversation**
  - **Delete Session**
- **Zero-Dependency Persistence**: Conversation trees and metadata are serialized to disk locally, allowing sessions to resume immediately upon restart.
