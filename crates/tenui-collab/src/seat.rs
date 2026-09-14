use std::collections::HashMap;

use tenui_core::{canvas::CanvasSubviewMut, color::Color, geometry::Point};

use crate::crdt::{CrdtOp, TextCrdt};

#[derive(Clone, Debug, PartialEq)]
pub struct SeatPresence {
    pub seat_id: u32,
    pub name: String,
    pub cursor_pos: Point,
    pub accent_color: Color,
    pub active_pane: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollabOperation {
    Insert {
        pos: usize,
        text: String,
        author: u32,
        clock: u64,
    },
    Delete {
        range: (usize, usize),
        author: u32,
        clock: u64,
    },
}

/// Tracks multi-seat presence (remote carets/badges) and a shared, convergent text buffer.
///
/// The document is backed by a [`TextCrdt`] (RGA), so the convergent wire path
/// ([`insert`](Self::insert)/[`delete`](Self::delete) to broadcast [`CrdtOp`]s,
/// [`apply_remote`](Self::apply_remote) to integrate them) is order-independent and
/// conflict-free across replicas.
///
/// The legacy positional [`apply_operation`](Self::apply_operation) is retained for
/// backward compatibility: it lowers a local, byte-offset [`CollabOperation`] into CRDT
/// ops against this replica's current text. It is a local-intent convenience, not a
/// convergent transport — send [`CrdtOp`]s between replicas.
pub struct MultiSeatManager {
    seats: HashMap<u32, SeatPresence>,
    doc: TextCrdt,
    clock: u64,
}

impl MultiSeatManager {
    /// Creates a manager with a default replica site of 1.
    ///
    /// For real multi-replica collaboration give each replica a distinct site via
    /// [`with_site`](Self::with_site), or ids collide and convergence breaks.
    pub fn new() -> Self {
        Self::with_site(1)
    }

    /// Creates a manager whose document CRDT uses the given replica `site`.
    pub fn with_site(site: u32) -> Self {
        Self {
            seats: HashMap::new(),
            doc: TextCrdt::new(site),
            clock: 0,
        }
    }

    pub fn add_seat(&mut self, presence: SeatPresence) {
        self.seats.insert(presence.seat_id, presence);
    }

    pub fn remove_seat(&mut self, seat_id: u32) -> Option<SeatPresence> {
        self.seats.remove(&seat_id)
    }

    pub fn get_seat(&self, seat_id: u32) -> Option<&SeatPresence> {
        self.seats.get(&seat_id)
    }

    pub fn seat_count(&self) -> usize {
        self.seats.len()
    }

    pub fn update_cursor(&mut self, seat_id: u32, pos: Point, pane_id: u64) {
        if let Some(seat) = self.seats.get_mut(&seat_id) {
            seat.cursor_pos = pos;
            seat.active_pane = pane_id;
        }
    }

    /// Renders other users' colored cursors and name badges onto a client's surface.
    /// Invariant: Current seat's own caret is never rendered as a remote caret.
    pub fn render_remote_seats(&self, current_seat_id: u32, surface: &mut CanvasSubviewMut) {
        let bounds = surface.bounds;
        for (id, seat) in &self.seats {
            if *id == current_seat_id {
                continue; // Do not render remote caret for local user
            }

            let cx = seat.cursor_pos.x;
            let cy = seat.cursor_pos.y;

            if cx < bounds.width && cy < bounds.height {
                // 1. Draw remote caret bar
                if let Some(cell) = surface.get_cell_mut(cx, cy) {
                    cell.bg = seat.accent_color;
                    cell.fg = Color::Black;
                }

                // 2. Draw floating user badge if row space permits
                if cy > 0 {
                    let badge = format!(" {} ", seat.name);
                    surface.write_str_clipped(cx, cy - 1, &badge, Color::White, seat.accent_color);
                }
            }
        }
    }

    /// Converts a byte offset into `s` to a visible character index, snapping offsets
    /// that fall inside a multi-byte sequence down to the enclosing char boundary.
    fn byte_to_visible(s: &str, byte: usize) -> usize {
        let b = byte.min(s.len());
        let mut boundary = b;
        while boundary > 0 && !s.is_char_boundary(boundary) {
            boundary -= 1;
        }
        s[..boundary].chars().count()
    }

    // ---- Convergent CRDT API (use this between replicas) ----

    /// Inserts `text` at a visible character index, returning [`CrdtOp`]s to broadcast.
    pub fn insert(&mut self, visible_index: usize, text: &str) -> Vec<CrdtOp> {
        self.clock += 1;
        self.doc.local_insert_str(visible_index, text)
    }

    /// Deletes `count` visible characters starting at `visible_index`, returning the
    /// [`CrdtOp`]s to broadcast.
    pub fn delete(&mut self, visible_index: usize, count: usize) -> Vec<CrdtOp> {
        self.clock += 1;
        let mut ops = Vec::new();
        for _ in 0..count {
            match self.doc.local_delete(visible_index) {
                Some(op) => ops.push(op),
                None => break,
            }
        }
        ops
    }

    /// Integrates a remote operation. Idempotent, commutative, and order-independent.
    pub fn apply_remote(&mut self, op: CrdtOp) {
        self.doc.apply(op);
    }

    // Legacy positional API (local intent; see the type-level note)

    /// Applies a byte-offset text operation to this replica's document and bumps the
    /// local clock, lowering it to convergent [`CrdtOp`]s under the hood.
    ///
    /// Offsets are snapped to char boundaries so multi-byte content can't panic. This is
    /// a local-intent convenience — send [`CrdtOp`]s (from [`insert`](Self::insert)/
    /// [`delete`](Self::delete)) to other replicas, not [`CollabOperation`].
    pub fn apply_operation(&mut self, op: CollabOperation) {
        self.clock += 1;
        let text = self.doc.text();
        match op {
            CollabOperation::Insert { pos, text: ins, .. } => {
                let vis = Self::byte_to_visible(&text, pos);
                self.doc.local_insert_str(vis, &ins);
            }
            CollabOperation::Delete {
                range: (start, end), ..
            } => {
                let vs = Self::byte_to_visible(&text, start);
                let ve = Self::byte_to_visible(&text, end).max(vs);
                for _ in vs..ve {
                    self.doc.local_delete(vs);
                }
            }
        }
    }

    /// The current visible document text.
    pub fn document_text(&self) -> String {
        self.doc.text()
    }

    /// Read-only access to the underlying convergent document.
    pub fn document(&self) -> &TextCrdt {
        &self.doc
    }

    pub fn current_clock(&self) -> u64 {
        self.clock
    }
}

impl Default for MultiSeatManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use tenui_core::{Buffer, Rect};

    use super::*;

    #[test]
    fn test_multi_seat_presence_and_isolation() {
        let mut manager = MultiSeatManager::new();
        manager.add_seat(SeatPresence {
            seat_id: 1,
            name: "Alice".into(),
            cursor_pos: Point { x: 5, y: 3 },
            accent_color: Color::Cyan,
            active_pane: 10,
        });
        manager.add_seat(SeatPresence {
            seat_id: 2,
            name: "Bob".into(),
            cursor_pos: Point { x: 12, y: 5 },
            accent_color: Color::Yellow,
            active_pane: 10,
        });

        assert_eq!(manager.seat_count(), 2);

        let mut buffer = Buffer::new(30, 10);
        let mut subview = buffer.subview_mut(Rect::new(0, 0, 30, 10));

        // When rendering for Alice (seat 1), only Bob's cursor should be injected
        manager.render_remote_seats(1, &mut subview);

        // Bob's caret at (12, 5) with badge at (12, 4)
        let bob_cell = buffer.get(12, 5).unwrap();
        assert_eq!(bob_cell.bg, Color::Yellow);

        let bob_badge_cell = buffer.get(13, 4).unwrap();
        assert_eq!(bob_badge_cell.bg, Color::Yellow);
        assert_eq!(bob_badge_cell.fg, Color::White);

        // Alice's own position (5, 3) should NOT have remote caret styling
        let alice_cell = buffer.get(5, 3).unwrap();
        assert_ne!(alice_cell.bg, Color::Cyan);
    }

    #[test]
    fn test_crdt_operations_and_clock() {
        let mut manager = MultiSeatManager::new();
        manager.apply_operation(CollabOperation::Insert {
            pos: 0,
            text: "Hello".into(),
            author: 1,
            clock: 1,
        });
        manager.apply_operation(CollabOperation::Insert {
            pos: 5,
            text: " World".into(),
            author: 2,
            clock: 2,
        });
        assert_eq!(manager.document_text(), "Hello World");
        assert_eq!(manager.current_clock(), 2);

        manager.apply_operation(CollabOperation::Delete {
            range: (5, 11),
            author: 1,
            clock: 3,
        });
        assert_eq!(manager.document_text(), "Hello");
        assert_eq!(manager.current_clock(), 3);
    }

    #[test]
    fn test_apply_operation_multibyte_no_panic() {
        let mut manager = MultiSeatManager::new();
        // "héllo" — 'é' is 2 bytes (0xC3 0xA9), so byte length is 6, not 5.
        manager.apply_operation(CollabOperation::Insert {
            pos: 0,
            text: "héllo".into(),
            author: 1,
            clock: 1,
        });

        // Offset 2 lands *inside* 'é' (boundary is at 1 and 3). Must snap, not panic.
        manager.apply_operation(CollabOperation::Insert {
            pos: 2,
            text: "X".into(),
            author: 2,
            clock: 2,
        });
        // Snapped down to byte 1 -> inserted right after 'h'.
        assert_eq!(manager.document_text(), "hXéllo");

        // Delete a range whose end bisects a multi-byte char — must not panic.
        manager.apply_operation(CollabOperation::Delete {
            range: (0, 3),
            author: 1,
            clock: 3,
        });
        assert!(manager.document_text().is_char_boundary(0));
    }

    #[test]
    fn test_manager_crdt_convergence_across_replicas() {
        // Two replicas with distinct sites, seeded with a shared base, then edited
        // concurrently and cross-delivered in different orders — must converge.
        let mut alice = MultiSeatManager::with_site(1);
        let mut bob = MultiSeatManager::with_site(2);

        let base = alice.insert(0, "Hi");
        for op in &base {
            bob.apply_remote(op.clone());
        }
        assert_eq!(bob.document_text(), "Hi");

        let a_ops = alice.insert(1, "AAA"); // between H and i
        let b_ops = bob.insert(1, "bbb"); // concurrently, same spot

        for op in &b_ops {
            alice.apply_remote(op.clone());
        }
        for op in a_ops.iter().rev() {
            bob.apply_remote(op.clone()); // reverse order on purpose
        }

        assert_eq!(alice.document_text(), bob.document_text());
        assert_eq!(alice.document().len(), 8); // Hi + AAA + bbb
    }
}
