#![forbid(unsafe_code)]
//! # Tenui Standard Library (`tenui-std`)
//!
//! Batteries-included widget catalog, accessible theming system, CIELAB color conversion,
//! and headless testing harness for the Tenui TUI framework.
//!
//! `tenui-std` provides production-ready components designed with mechanical sympathy:
//! zero-allocation viewport slicing, high-performance virtualized scrolling, streaming
//! tokenization, and differential rendering compatibility.
//!
//! ## Widget Catalog
//!
//! Components are grouped into functional families:
//!
//! ### Text & Code
//! - [`TextInput`]: Single-line text input with full cursor navigation, selection, and OS clipboard integration.
//! - [`TextEditor`]: Multi-line code/prose editor with multi-cursor support, branching undo trees, and substring search.
//! - [`RopeEditor`]: Rope-backed multi-line editor for efficient arbitrarily large file editing.
//! - [`TextView`]: Read-only formatted text viewer with scrolling and text wrapping.
//! - [`CodeView`]: Viewport-bounded syntax-highlighted code viewer supporting Rust, Python, JavaScript, JSON, and Markdown.
//! - [`MarkdownView`]: GFM/CommonMark markdown renderer with headers, lists, inline formatting, code blocks, and typeset math.
//! - [`DiffView`]: Myers differential viewer supporting side-by-side split and unified modes with intra-line character highlighting.
//! - [`Banner`]: Large multi-cell stylized banner headlines with half-block and full-block font rendering.
//! - [`BrailleCanvas`]: High-resolution 2×4 sub-pixel Braille canvas with line and micro-text plotting.
//! - [`UnicodeFont`]: Zero-allocation typographic font transformations (Monospace, Sans, Blackboard Bold, SmallCaps, Fraktur, etc.).
//! - [`Icon`]: Standardized development, file, and UI icons with automatic Nerd Font, Unicode, and ASCII fallbacks.
//!
//! ### Data & Virtualization
//! - [`VirtualTable`]: High-performance virtualized data table rendering datasets with 100,000+ rows in sub-millisecond frame times.
//! - [`TreeView`]: Hierarchical collapsable tree view with depth indentation and node selection.
//! - [`DiagnosticView`]: Compiler-grade diagnostic renderer with multi-span squiggles, inline notes, and severity coloring.
//!
//! ### Charts & Metrics
//! - [`LinePlot`], [`BarChart`], [`CandlestickChart`]: Interactive responsive data charts with multiple coordinate systems.
//! - [`SmoothProgressBar`], [`Sparkline`], [`Spinner`]: Micro-visualizations with sub-cell Unicode block rendering.
//! - [`MultiProgress`]: Multi-task concurrent progress display with task slot status indicators.
//!
//! ### Interactive Shell & Overlays
//! - [`ChatThread`]: Conversational LLM chat interface with streaming message chunks, collapsible thinking cards, and tool call inspection.
//! - [`CommandPalette`]: Quick-open fuzzy command launcher with keybinding cues and category filtering.
//! - [`Modal`]: Centered overlay dialog with focus trapping and backdrop dimming.
//! - [`Popover`]: Contextual floating popup placed relative to anchor coordinates.
//! - [`WhichKey`]: Keymap hint popup guiding users through multi-key hotkey sequences.
//! - [`FilePicker`]: Interactive filesystem directory browser and file selector.
//! - [`FileDropZone`]: Drag-and-drop target zone visualizer.
//! - [`LogStreamer`]: Auto-scrolling ring-buffered log console with severity level filtering.
//!
//! ### System & Sandboxing
//! - [`TerminalPane`]: Full VT100 / ANSI virtual terminal emulator running an interactive PTY subprocess.
//! - [`SubprocessView`]: Out-of-process crash-isolated pane executor.
//! - [`ConfigWatcher`]: Live configuration reloader monitoring filesystem changes.
//!
//! ### Theming & Color Science
//! - [`ThemePalette`]: Semantic design tokens (`surface`, `accent`, `text_primary`, etc.) for consistent dark/light themes.
//! - [`contrast_ratio`], [`ensure_contrast`]: Automated WCAG 2.1 contrast ratio verification between foreground and background colors.
//! - [`rgb_to_cielab`], [`downsample_to_ansi256`], [`downsample_to_ansi16`]: Perceptually uniform CIELAB $\Delta E^*$ color mapping for legacy terminals.
//!
//! ### Headless Testing
//! - [`TenuiTestHarness`]: Headless terminal harness allowing zero-TTY UI testing, screenshot assertions, and CI verification.
//!
//! ## Runnable Example: Virtual Table in Headless Harness
//!
//! ```rust
//! use tenui_core::Rect;
//! use tenui_std::{Column, TenuiTestHarness, VirtualTable};
//!
//! struct Process {
//!     pid: u32,
//!     name: &'static str,
//!     cpu: f32,
//! }
//!
//! let processes = vec![
//!     Process { pid: 1, name: "systemd", cpu: 0.1 },
//!     Process { pid: 42, name: "tenui", cpu: 1.4 },
//! ];
//!
//! let columns = vec![
//!     Column::new("PID", 6, |p: &Process| p.pid.to_string()),
//!     Column::new("Name", 12, |p: &Process| p.name.to_string()),
//!     Column::new("CPU %", 8, |p: &Process| format!("{:.1}%", p.cpu)),
//! ];
//!
//! let table = VirtualTable::new(&processes, columns);
//!
//! let mut harness = TenuiTestHarness::new(40, 10);
//! harness.draw_buffer(|back| {
//!     let mut subview = back.subview_mut(Rect::new(0, 0, 40, 10));
//!     table.render(&mut subview);
//! });
//!
//! assert_eq!(harness.get_text_at(0, 0, 3), "PID");
//! ```

pub mod calendar;
pub mod chart;
pub mod cielab;
pub mod color_picker;
pub mod config;
pub mod context_menu;
pub mod diag;
pub mod doc;
pub mod form;
pub mod harness;
pub mod input;
pub mod layout_widgets;
pub mod overlay;
pub mod popover;
pub mod pty;
pub mod sandbox;
pub mod status_bar;
pub mod syntax;
pub mod tab_bar;
pub mod table;
pub mod toast;

pub mod chat_thread;
pub mod drop_zone;
pub mod file_picker;
pub mod font;
pub mod icons;
pub mod log_streamer;
pub mod media;
pub mod multi_progress;
pub mod prompt;
pub mod rich;
pub mod selection;
pub mod text_editor;
pub mod text_table;
pub mod text_view;
pub mod theme;
pub mod tree_view;
pub mod viz;
pub mod which_key;

pub use calendar::{Calendar, Date};
pub use chart::{
    BarChart, BarItem, BarOrientation, Candle, CandlestickChart, Heatmap, LinePlot, PlotCanvasType, ScatterPlot,
    Series, StackedBarChart, StackedSegment,
};
pub use chat_thread::{ChatMessage, ChatThread, MessageRole, ThinkingBlock, ToolCallCard};
pub use cielab::{
    ColorProfile, Lab, detect_color_capability, downsample_to_ansi16, downsample_to_ansi256, rgb_to_cielab,
};
pub use color_picker::{ColorPicker, Hsv};
pub use config::ConfigWatcher;
pub use context_menu::{ContextMenu, MenuItem};
pub use diag::{Diagnostic, DiagnosticLabel, DiagnosticSeverity, DiagnosticView};
pub use doc::{
    AlignedDiffLine, ChangeType, DiffHunk, DiffLine, DiffMode, DiffView, MarkdownBlock, MarkdownRenderer, MarkdownView,
    MyersDiff, compute_diff_async, parse_inline_markdown, zip_aligned_diff,
};
pub use drop_zone::FileDropZone;
pub use file_picker::{FileEntry, FilePicker};
pub use font::{Banner, BannerAlignment, BigFontStyle, BrailleCanvas, UnicodeFont, bitmap4x6};
pub use form::{Form, FormField, FormFieldKind, Wizard, WizardStep};
pub use harness::{TenuiTestHarness, VirtualTerminal};
pub use icons::{Icon, IconMode};
pub use input::{RopeEditor, TextInput};
pub use layout_widgets::{DockContainer, ScrollArea, SplitOrientation, SplitPane};
pub use log_streamer::LogStreamer;
pub use media::{GraphicsProtocol, ImageCanvas};
pub use multi_progress::{MultiProgress, TaskSlot, TaskStatus};
pub use overlay::{CommandItem, CommandPalette, Modal};
pub use popover::{Popover, PopoverPlacement};
pub use prompt::{ReplInput, SuggestionItem};
pub use pty::TerminalPane;
pub use rich::{RichSpan, wrap_rich_spans, wrap_rich_spans_cont};
pub use sandbox::{SubprocessState, SubprocessView};
pub use selection::{DocumentSelection, SelectableRow, slice_by_cols};
pub use status_bar::{StatusBar, StatusSegment};
pub use syntax::{CodeView, Language, LineTokenizer, SyntaxTheme, TokenKind, TokenSpan, render_code_line};
pub use tab_bar::{Tab, TabBar};
pub use table::{Column, VirtualTable};
pub use text_editor::{TextEditor, VisLine};
pub use text_table::{TableAlignment, TableBorder, TextTable};
pub use text_view::TextView;
pub use theme::{ThemePalette, contrast_ratio, ensure_contrast, readable_on, relative_luminance};
pub use toast::{Toast, ToastLevel, ToastPosition, ToastStack};
pub use tree_view::{TreeNode, TreeView};
pub use viz::{SmoothProgressBar, Sparkline, Spinner, SpinnerStyle};
pub use which_key::{KeyAction, WhichKey};
