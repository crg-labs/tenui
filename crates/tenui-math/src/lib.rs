#![forbid(unsafe_code)]
//! # Tenui Math (`tenui-math`)
//!
//! Sub-cell mathematical typesetting and proportional box layout subsystem.
//!
//! `tenui-math` provides a TeX-inspired mathematical layout engine that decomposes
//! complex expressions into 2D character-cell grids:
//!
//! ## Core Primitives
//!
//! - **[`MathNode`]**: Abstract syntax tree representing mathematical structures:
//!   - Fractions ([`MathNode::fraction`]) with centered horizontal division bars.
//!   - Superscripts and Subscripts ([`MathNode::supsub`]) with proper baseline alignment.
//!   - Integrals ([`MathNode::integral`]) with multi-row bracket arcs (`⌠`, `│`, `⌡`) and limits.
//!   - Matrices ([`MathNode::matrix`]) with aligned columns and enclosing delimiters.
//!   - Text and Symbols ([`MathNode::text`], [`MathNode::symbol`]).
//! - **[`MathTypesetter`]**: Evaluator that measures bounding box dimensions ([`MathTypesetter::measure`])
//!   and renders formatted typography into canvas subviews.
//!
//! ## Runnable Example: Typesetting a Fraction
//!
//! ```rust
//! use tenui_math::{MathNode, MathTypesetter};
//!
//! let typesetter = MathTypesetter;
//!
//! // Construct: (x + 1) / 2
//! let expr = MathNode::fraction(
//!     MathNode::text("x + 1"),
//!     MathNode::text("2"),
//! );
//!
//! let (width, height) = typesetter.measure(&expr);
//! assert_eq!(width, 7);
//! assert_eq!(height, 3);
//! ```

pub mod parse;
pub mod typeset;

pub use parse::{latex_to_unicode, parse_latex, to_subscript_str, to_superscript_str};
pub use typeset::{MathNode, MathTypesetter};
