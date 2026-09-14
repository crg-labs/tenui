#![forbid(unsafe_code)]
//! # Tenui Compositor (`tenui-compositor`)
//!
//! Off-screen spatial layer compositing, occlusion culling, sub-cell vector graphics, and spatial navigation.
//!
//! `tenui-compositor` operates as Tenui's graphics and spatial composition subsystem,
//! providing three key capabilities:
//!
//! ## 1. Z-Indexed Layer Compositing & Occlusion Culling
//!
//! The [`Compositor`] manages overlapping visual layers (such as modals, tooltips, popovers,
//! and floating palettes). Before painting lower layers, it calculates rectangular difference
//! cuts ([`cut_rect`], [`cut_multi`]) against all obscuring higher-Z layers. This completely
//! avoids rendering cells that would be immediately overwritten, eliminating back-buffer overdraw.
//!
//! ## 2. High-Density Sub-Cell Vector Graphics
//!
//! Modern terminals support Unicode matrices that multiply effective graphical resolution:
//! - **[`BrailleCanvas`]**: $2 \times 4$ dot matrix per cell (Unicode `U+2800..U+28FF`), multiplying
//!   horizontal resolution by $2\times$ and vertical resolution by $4\times$. Includes integer Bresenham
//!   line and midpoint circle rasterizers.
//! - **[`HalfBlockCanvas`]**: $1 \times 2$ pixel matrix per cell (`▀`, `▄`) with 24-bit TrueColor
//!   foreground and background blending and Xiaolin Wu anti-aliased line drawing.
//! - **[`QuadrantCanvas`]**: $2 \times 2$ block matrix per cell (`▖`, `▗`, `▘`, `▝`, `▞`, `▟`, etc.).
//!
//! ## 3. Anisotropic Spatial Navigation
//!
//! The [`SpatialNav`] engine implements 2D geometric focus movement across nodes ([`FocusNode`])
//! using keyboard arrow keys. Because terminal character cells are approximately twice as tall
//! as they are wide ($\approx 1:2$ aspect ratio), `SpatialNav` weights vertical Euclidean distances
//! with an anisotropic coefficient ($k = 2.0$), preventing unintended diagonal jumping.
//!
//! ## Runnable Example: Sub-Cell Braille Plotting
//!
//! ```rust
//! use tenui_compositor::BrailleCanvas;
//! use tenui_core::{Buffer, Color, Rect};
//!
//! // Create a 10x5 character cell canvas (giving 20x20 sub-pixel resolution)
//! let mut canvas = BrailleCanvas::new(10, 5);
//!
//! // Draw a diagonal line in sub-pixel coordinates
//! canvas.draw_line(0, 0, 19, 19);
//!
//! // Render to a tenui buffer subview
//! let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 5));
//! let mut subview = buffer.subview_mut(Rect::new(0, 0, 10, 5));
//! canvas.render_to_subview(&mut subview, Color::Green, Color::Reset);
//!
//! // Verify top-left cell has braille dots set
//! let cell = buffer.get(0, 0).unwrap();
//! assert_ne!(cell.symbol.as_str(), " ");
//! ```

pub mod braille;
pub mod canvas;
pub mod cut;
pub mod halfblock;
pub mod layer;
pub mod quadrant;
pub mod spatial_nav;

pub use braille::BrailleCanvas;
pub use canvas::IsolatedCanvas;
pub use cut::{cut_multi, cut_rect};
pub use halfblock::HalfBlockCanvas;
pub use layer::{Compositor, InvalidationSink, LayerCompositor, ModalHandle};
pub use quadrant::QuadrantCanvas;
pub use spatial_nav::{Direction, FocusNode, SpatialNav};
