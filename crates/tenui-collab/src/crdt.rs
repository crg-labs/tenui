//! Convergent sequence CRDT for collaborative text (RGA / causal tree).
//!
//! Each character is an immutable element with a globally unique [`OpId`]. An insert
//! references the element it was placed *after* (`origin`); a delete tombstones an
//! element by id. The document order is the causal-tree preorder with concurrent
//! siblings ordered by id descending — a deterministic function of the operation *set*,
//! so replicas that apply the same operations converge to the same text regardless of
//! arrival order.
//!
//! Guarantees:
//! - **Commutative & convergent:** applying the same ops in any order yields the same text.
//! - **Idempotent:** applying an op twice is a no-op ([`applied`](TextCrdt) tracks inserts;
//!   deletes are naturally idempotent).
//! - **Order-independent delivery:** an op whose `origin`/target has not arrived yet is
//!   buffered and retried, so causal delivery is not required by callers.
//!
//! Like all RGA-family CRDTs, concurrent inserts at the same point may interleave; this
//! implementation guarantees *convergence*, not interleaving-avoidance (cf. Fugue/YATA).

use std::collections::HashSet;

/// Globally unique, totally ordered operation identifier.
///
/// Ordered by `(counter, site)` so concurrent operations break ties deterministically.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OpId {
    pub counter: u64,
    pub site: u32,
}

/// A collaborative text operation exchanged between replicas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CrdtOp {
    /// Insert `ch` immediately after the element identified by `origin` (or at the
    /// document start when `origin` is `None`).
    Insert { id: OpId, origin: Option<OpId>, ch: char },
    /// Tombstone the element identified by `id`.
    Delete { id: OpId },
}

#[derive(Clone, Debug)]
struct Elem {
    id: OpId,
    origin: Option<OpId>,
    ch: char,
    deleted: bool,
}

/// A replicated, convergent text buffer.
#[derive(Clone, Debug)]
pub struct TextCrdt {
    site: u32,
    counter: u64,
    /// Elements in CRDT linear order (visible glyphs and tombstones interleaved).
    // ponytail: linear `index_of` scan → O(n) per op; fine for editor/chat sized docs,
    // swap for an order-statistics tree if multi-megabyte documents ever matter.
    elements: Vec<Elem>,
    /// Ids of inserts already integrated (idempotency guard).
    applied: HashSet<OpId>,
    /// Ops awaiting a not-yet-present `origin`/target (delivery-order independence).
    pending: Vec<CrdtOp>,
}

impl TextCrdt {
    /// Creates an empty document for the replica identified by `site`.
    ///
    /// Every replica must use a distinct `site`, or ids collide and convergence breaks.
    pub fn new(site: u32) -> Self {
        Self {
            site,
            counter: 0,
            elements: Vec::new(),
            applied: HashSet::new(),
            pending: Vec::new(),
        }
    }

    /// The replica's site identifier.
    pub fn site(&self) -> u32 {
        self.site
    }

    /// The visible document text.
    pub fn text(&self) -> String {
        self.elements.iter().filter(|e| !e.deleted).map(|e| e.ch).collect()
    }

    /// Number of visible characters.
    pub fn len(&self) -> usize {
        self.elements.iter().filter(|e| !e.deleted).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn next_id(&mut self) -> OpId {
        self.counter += 1;
        OpId {
            counter: self.counter,
            site: self.site,
        }
    }

    /// Lamport-style clock bump so locally generated ids sort after everything seen.
    fn observe(&mut self, id: OpId) {
        if id.counter > self.counter {
            self.counter = id.counter;
        }
    }

    fn index_of(&self, id: OpId) -> Option<usize> {
        self.elements.iter().position(|e| e.id == id)
    }

    /// The id of the `vis`-th visible element, if it exists.
    fn visible_id_at(&self, vis: usize) -> Option<OpId> {
        self.elements.iter().filter(|e| !e.deleted).nth(vis).map(|e| e.id)
    }

    /// Inserts `ch` at visible index `vis`, returning the op to broadcast.
    ///
    /// `vis` is clamped to `[0, len()]`; `len()` appends. The returned [`CrdtOp`] should be
    /// broadcast to peers for convergence (safe to drop for single-replica editing).
    pub fn local_insert(&mut self, vis: usize, ch: char) -> CrdtOp {
        let origin = if vis == 0 {
            None
        } else {
            // Insert *after* the (vis-1)-th visible element.
            self.visible_id_at(vis - 1)
                .or_else(|| self.elements.last().map(|e| e.id))
        };
        let id = self.next_id();
        let op = CrdtOp::Insert { id, origin, ch };
        self.apply(op.clone());
        op
    }

    /// Inserts each char of `s` starting at visible index `vis`, returning the ops in order.
    pub fn local_insert_str(&mut self, vis: usize, s: &str) -> Vec<CrdtOp> {
        let mut ops = Vec::new();
        for (i, ch) in s.chars().enumerate() {
            ops.push(self.local_insert(vis + i, ch));
        }
        ops
    }

    /// Deletes the visible element at index `vis`, returning the op to broadcast (or
    /// `None` when `vis` is out of range).
    pub fn local_delete(&mut self, vis: usize) -> Option<CrdtOp> {
        let id = self.visible_id_at(vis)?;
        let op = CrdtOp::Delete { id };
        self.apply(op.clone());
        Some(op)
    }

    /// Applies a (local or remote) operation. Idempotent and commutative; ops that are
    /// not yet causally ready are buffered and retried automatically.
    pub fn apply(&mut self, op: CrdtOp) {
        if self.try_apply(op) {
            self.drain_pending();
        }
    }

    /// Attempts to apply one op. Returns `true` if it was integrated (or was a harmless
    /// duplicate), `false` if it was buffered pending a missing dependency.
    fn try_apply(&mut self, op: CrdtOp) -> bool {
        match op {
            CrdtOp::Insert { id, origin, ch } => {
                if self.applied.contains(&id) {
                    return true; // duplicate — idempotent
                }
                // Dependency: origin must already be present (None = document start).
                if let Some(o) = origin
                    && self.index_of(o).is_none()
                {
                    self.pending.push(CrdtOp::Insert { id, origin, ch });
                    return false;
                }
                self.integrate(Elem {
                    id,
                    origin,
                    ch,
                    deleted: false,
                });
                self.applied.insert(id);
                self.observe(id);
                true
            }
            CrdtOp::Delete { id } => match self.index_of(id) {
                Some(idx) => {
                    self.elements[idx].deleted = true; // idempotent
                    self.observe(id);
                    true
                }
                None => {
                    self.pending.push(CrdtOp::Delete { id });
                    false
                }
            },
        }
    }

    /// Retries buffered ops until no further progress is possible.
    fn drain_pending(&mut self) {
        loop {
            let ready = std::mem::take(&mut self.pending);
            let before = self.applied.len() + self.tombstone_count();
            for op in ready {
                self.try_apply(op);
            }
            let after = self.applied.len() + self.tombstone_count();
            // Stop when a full pass integrated nothing new.
            if self.pending.is_empty() || after == before {
                break;
            }
        }
    }

    fn tombstone_count(&self) -> usize {
        self.elements.iter().filter(|e| e.deleted).count()
    }

    /// RGA integration: place `elem` after its origin, before any concurrent sibling with
    /// a smaller id (siblings sort by id descending). See module docs for convergence.
    fn integrate(&mut self, elem: Elem) {
        let start = match elem.origin {
            None => 0,
            Some(o) => self.index_of(o).map(|i| i + 1).unwrap_or(0),
        };
        let mut i = start;
        while i < self.elements.len() {
            // Skip past any already-present element with a higher id (a concurrent
            // insertion that sorts before us); stop at the first with a lower id.
            if self.elements[i].id > elem.id {
                i += 1;
            } else {
                break;
            }
        }
        self.elements.insert(i, elem);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(site: u32, s: &str) -> TextCrdt {
        let mut d = TextCrdt::new(site);
        d.local_insert_str(0, s);
        d
    }

    #[test]
    fn test_local_editing_reads_back() {
        let mut d = typed(1, "helo");
        assert_eq!(d.text(), "helo");
        d.local_insert(3, 'l'); // "hello"
        assert_eq!(d.text(), "hello");
        d.local_delete(0); // "ello"
        assert_eq!(d.text(), "ello");
        assert_eq!(d.len(), 4);
    }

    #[test]
    fn test_idempotent_apply() {
        let mut a = TextCrdt::new(1);
        let ops = a.local_insert_str(0, "hi");
        let mut b = TextCrdt::new(2);
        for op in &ops {
            b.apply(op.clone());
            b.apply(op.clone()); // duplicate delivery
        }
        assert_eq!(b.text(), "hi");
    }

    #[test]
    fn test_out_of_order_delivery() {
        let mut a = TextCrdt::new(1);
        let ops = a.local_insert_str(0, "abc"); // three inserts, each depends on the last
        let mut b = TextCrdt::new(2);
        // Deliver in reverse — every op arrives before its origin.
        for op in ops.iter().rev() {
            b.apply(op.clone());
        }
        assert_eq!(b.text(), "abc");
    }

    /// The core property: concurrent edits converge regardless of application order.
    #[test]
    fn test_concurrent_convergence() {
        // Shared prefix both replicas start from.
        let mut base = TextCrdt::new(1);
        let base_ops = base.local_insert_str(0, "XY");

        let mut alice = TextCrdt::new(10);
        let mut bob = TextCrdt::new(20);
        for op in &base_ops {
            alice.apply(op.clone());
            bob.apply(op.clone());
        }
        assert_eq!(alice.text(), "XY");
        assert_eq!(bob.text(), "XY");

        // Concurrent edits: Alice inserts "abc" after X (pos 1); Bob inserts "123" after X.
        let alice_ops = alice.local_insert_str(1, "abc");
        let bob_ops = bob.local_insert_str(1, "123");

        // Cross-deliver: Alice gets Bob's ops in order, Bob gets Alice's reversed.
        for op in &bob_ops {
            alice.apply(op.clone());
        }
        for op in alice_ops.iter().rev() {
            bob.apply(op.clone());
        }

        assert_eq!(
            alice.text(),
            bob.text(),
            "replicas diverged: {:?} vs {:?}",
            alice.text(),
            bob.text()
        );
        // Both edits are present and the shared anchors survive.
        assert!(alice.text().starts_with('X'));
        assert!(alice.text().ends_with('Y'));
        assert_eq!(alice.len(), 8); // XY + abc + 123
    }

    /// Stronger: many replicas, many ops, shuffled delivery orders all converge.
    #[test]
    fn test_convergence_under_shuffled_delivery() {
        // Build a pool of ops from three authors editing concurrently from a shared base.
        let mut base = TextCrdt::new(1);
        let base_ops = base.local_insert_str(0, "root");

        let mut authors: Vec<TextCrdt> = (0..3).map(|i| TextCrdt::new(100 + i)).collect();
        for d in &mut authors {
            for op in &base_ops {
                d.apply(op.clone());
            }
        }

        let mut all_ops = base_ops.clone();
        // Each author makes a few concurrent edits at varying positions.
        for (i, d) in authors.iter_mut().enumerate() {
            let ops = d.local_insert_str(i % 4, &format!("<{}>", i));
            all_ops.extend(ops);
            if let Some(op) = d.local_delete(0) {
                all_ops.push(op);
            }
        }

        // Deterministic pseudo-random shuffles (no rand dep): rotate + reverse variants.
        let make_order = |seed: usize| -> Vec<CrdtOp> {
            let mut v = all_ops.clone();
            v.rotate_left(seed % all_ops.len().max(1));
            if seed.is_multiple_of(2) {
                v.reverse();
            }
            v
        };

        let mut reference: Option<String> = None;
        for seed in 0..12 {
            let mut replica = TextCrdt::new(9000 + seed as u32);
            for op in make_order(seed) {
                replica.apply(op);
            }
            match &reference {
                None => reference = Some(replica.text()),
                Some(r) => assert_eq!(
                    &replica.text(),
                    r,
                    "seed {} diverged: {:?} vs reference {:?}",
                    seed,
                    replica.text(),
                    r
                ),
            }
        }
        assert!(reference.is_some());
    }
}
