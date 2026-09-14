#![forbid(unsafe_code)]
//! # Tenui Layout (`tenui-layout`)
//!
//! Pure-Rust immediate-mode flexbox and grid layout engine built on [Taffy](https://github.com/DioxusLabs/taffy).
//!
//! `tenui-layout` allows terminal applications to define responsive, dynamic interfaces
//! using standard CSS Flexbox and CSS Grid semantics, while rendering directly to
//! [`tenui_core::Buffer`] without allocating an intermediate retained scene graph.
//!
//! ## Core Capabilities
//!
//! - **Immediate-Mode Ergonomics**: Define UI hierarchy per-frame via [`Ui`] builder methods
//!   ([`Ui::row`], [`Ui::column`], [`Ui::grid`], [`Ui::leaf`], [`Ui::block`]).
//! - **Standard CSS Layout Semantics**: Full support for flex direction, alignment, justification,
//!   gap, padding, margin, min/max bounds, and fractional grow/shrink factors via [`StyleExt`].
//! - **Border & Divider Primitives**: Built-in Unicode border styling ([`BorderStyle`]),
//!   including single, double, rounded, heavy, dashed, and split header lines.
//! - **Zero TTY Dependency**: Renders seamlessly onto any [`tenui_core::Terminal`] backend
//!   or buffer via the [`TerminalLayoutExt`] extension trait.
//!
//! ## Runnable Example
//!
//! ```rust
//! use tenui_core::{Color, Modifier, Terminal};
//! use tenui_layout::{BorderStyle, Style, StyleExt, TerminalLayoutExt};
//!
//! let mut terminal = Terminal::with_writer(std::io::Cursor::new(Vec::new()), 60, 10);
//!
//! terminal.draw_layout(|ui| {
//!     ui.row(Style::default().flex_row(), |ui| {
//!         ui.leaf(Style::default().width_cells(20.0), |subview| {
//!             subview.set_string(0, 0, "Sidebar", Color::Cyan, Color::Reset, Modifier::BOLD);
//!         });
//!         ui.block(
//!             Style::default().grow(1.0),
//!             Some("Main View"),
//!             BorderStyle::ROUNDED,
//!             Color::Green,
//!             Color::Reset,
//!             |ui| {
//!                 ui.text(Style::default(), "Hello Tenui!", Color::White, Color::Reset, Modifier::empty());
//!             },
//!         );
//!     });
//! }).expect("layout render succeeds");
//!
//! assert_eq!(terminal.front().get(0, 0).unwrap().symbol.as_str(), "S");
//! ```

pub mod border;
pub mod style;
pub mod ui;

use std::io::{self, Write};

pub use border::{
    BorderStyle, draw_border, draw_border_with_header, draw_border_with_title, draw_horizontal_divider,
    draw_split_header,
};
pub use style::{StyleExt, *};
use tenui_core::Terminal;
pub use ui::Ui;

/// Extension trait providing immediate-mode Taffy layout capabilities to `Terminal`.
pub trait TerminalLayoutExt {
    fn draw_layout<'frame, F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Ui<'frame>);

    fn draw_layout_with_bg<'frame, F>(&mut self, bg: tenui_core::Color, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Ui<'frame>);
}

impl<W: Write> TerminalLayoutExt for Terminal<W> {
    fn draw_layout<'frame, F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Ui<'frame>),
    {
        self.draw_layout_with_bg(tenui_core::Color::Reset, f)
    }

    fn draw_layout_with_bg<'frame, F>(&mut self, bg: tenui_core::Color, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Ui<'frame>),
    {
        let mut ui = Ui::<'frame>::new();
        f(&mut ui);
        self.draw_buffer_with_bg(bg, |back| {
            ui.render_to_buffer(back);
        })
    }
}
