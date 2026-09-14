//! Resilience soak tests: hammer every input / parse / diff / CRDT / editing surface with
//! randomized and hostile data and assert no panics + core invariants. Complements the
//! showcase `--bench` harness with deterministic, CI-runnable coverage.

use tenui::{
    collab::{CrdtOp, TextCrdt},
    core::{
        Buffer, Cell, Color, Modifier, PathSanitizer, SgrCoalescer, SgrMouseParser, UnderlineStyle, input::InputDemuxer,
    },
    text::TextBuffer,
};

/// Deterministic PRNG (reproducible failures; no rand dep — keeps the crate lightweight).
struct Rng(u64);
impl Rng {
    fn u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn range(&mut self, n: usize) -> usize {
        (self.u32() as usize) % n.max(1)
    }
    fn byte(&mut self) -> u8 {
        (self.u32() & 0xFF) as u8
    }
}

fn random_color(rng: &mut Rng) -> Color {
    match rng.range(5) {
        0 => Color::Reset,
        1 => Color::Indexed(rng.byte()),
        2 => Color::Rgb(rng.byte(), rng.byte(), rng.byte()),
        3 => Color::White,
        _ => Color::Black,
    }
}

fn random_symbol(rng: &mut Rng) -> &'static str {
    // Mix ASCII, wide CJK, combining/emoji, and a long spillover EGC.
    const SYMS: [&str; 8] = ["a", " ", "界", "🦀", "é", "👨‍👩‍👧‍👦", "│", "▀"];
    SYMS[rng.range(SYMS.len())]
}

fn random_cell(rng: &mut Rng) -> Cell {
    let mut c = Cell::new(random_symbol(rng));
    c.fg = random_color(rng);
    c.bg = random_color(rng);
    if rng.u32().is_multiple_of(2) {
        c.modifier = Modifier::BOLD | Modifier::ITALIC;
    }
    c.underline_style = if rng.u32().is_multiple_of(3) {
        UnderlineStyle::Curly
    } else {
        UnderlineStyle::None
    };
    c.underline_color = random_color(rng);
    c
}

fn fill_random(buf: &mut Buffer, rng: &mut Rng) {
    let n = buf.cells.len();
    for _ in 0..(n / 3) {
        let i = rng.range(n);
        buf.cells[i] = random_cell(rng);
    }
}

#[test]
fn sgr_coalescer_survives_random_buffers() {
    let mut rng = Rng(0x50FA);
    let (w, h) = (80u16, 24u16);
    let mut front = Buffer::new(w, h);
    let mut back = Buffer::new(w, h);
    let mut coalescer = SgrCoalescer::new();

    for round in 0..2000 {
        // Occasionally poison (forces full re-emit) and resize-like clears.
        if round % 97 == 0 {
            front.poison_all();
            coalescer.reset_state();
        }
        fill_random(&mut back, &mut rng);
        let mut out = Vec::with_capacity(4096);
        coalescer.write_diff(&mut out, &front, &back).expect("write_diff");
        // The stream must be valid UTF-8 and synchronized-output bracketed.
        let s = String::from_utf8(out).expect("valid utf8 diff");
        assert!(s.starts_with("\x1b[?2026h") && s.ends_with("\x1b[?2026l"));
        std::mem::swap(&mut front, &mut back);
    }
}

#[test]
fn parsers_never_panic_on_hostile_input() {
    let mut rng = Rng(0xBEEF);
    let mut demux = InputDemuxer::new();
    let alphabet = b"\x1b[<0123456789;Mm~file:/%2F\\ \nhome\xff";
    for _ in 0..30_000 {
        let len = rng.range(48);
        let bytes: Vec<u8> = (0..len).map(|_| alphabet[rng.range(alphabet.len())]).collect();
        let s = String::from_utf8_lossy(&bytes);
        let _ = SgrMouseParser::parse_sgr(&s);
        let _ = PathSanitizer::parse_drop_payload(&s);
        let _ = demux.feed_bytes(&bytes);
    }
}

#[test]
fn text_buffer_random_edits_never_panic_and_undo_unwinds() {
    let mut rng = Rng(0xED17);
    let mut buf = TextBuffer::new();
    for _ in 0..5000 {
        match rng.range(7) {
            0 | 1 => buf.insert(random_symbol(&mut rng)),
            2 => buf.insert("\n"),
            3 => buf.backspace(),
            4 => buf.move_left(rng.u32().is_multiple_of(2)),
            5 => buf.move_right(rng.u32().is_multiple_of(2)),
            _ => {
                buf.move_up(false);
                buf.move_down(false);
            }
        }
        // Text must always remain valid UTF-8 with the cursor on a boundary (implicit:
        // no panic from the ops themselves proves boundary safety).
        let _ = buf.text().len();
    }
    // Undoing every committed edit returns to the initial empty state (the branching undo
    // tree is finite; `undo()` returns false at the root).
    while buf.undo() {}
    assert_eq!(buf.text(), "", "undo history must unwind to the initial empty state");
}

#[test]
fn crdt_converges_under_random_concurrent_ops() {
    let mut rng = Rng(0xC0DE);
    // Three replicas edit concurrently from a shared base; collect every op.
    let mut base = TextCrdt::new(1);
    let mut all: Vec<CrdtOp> = base.local_insert_str(0, "shared-root");

    let mut replicas: Vec<TextCrdt> = (0..3).map(|i| TextCrdt::new(10 + i)).collect();
    for r in &mut replicas {
        for op in &all {
            r.apply(op.clone());
        }
    }
    for r in &mut replicas {
        for _ in 0..40 {
            let len = r.len();
            if len > 0 && rng.u32().is_multiple_of(4) {
                if let Some(op) = r.local_delete(rng.range(len)) {
                    all.push(op);
                }
            } else {
                let pos = rng.range(len + 1);
                let ch = (b'a' + (rng.u32() % 26) as u8) as char;
                all.push(r.local_insert(pos, ch));
            }
        }
    }

    // Deliver the full op set to fresh replicas in several shuffled orders — all converge.
    let make_order = |seed: usize| {
        let mut v = all.clone();
        let len = v.len().max(1);
        v.rotate_left(seed * 7 % len);
        if seed.is_multiple_of(2) {
            v.reverse();
        }
        v
    };
    let mut reference: Option<String> = None;
    for seed in 0..16 {
        let mut rep = TextCrdt::new(9000 + seed as u32);
        for op in make_order(seed) {
            rep.apply(op);
        }
        match &reference {
            None => reference = Some(rep.text()),
            Some(r) => assert_eq!(&rep.text(), r, "CRDT diverged for delivery order {}", seed),
        }
    }
    assert!(reference.is_some());
}
