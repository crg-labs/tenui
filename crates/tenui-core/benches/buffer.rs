use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use tenui_core::{Buffer, Cell, Color, Modifier, Rect};

fn bench_buffer_new(c: &mut Criterion) {
    c.bench_function("buffer_new_80x24", |b| {
        b.iter(|| Buffer::new(black_box(80), black_box(24)));
    });
    c.bench_function("buffer_new_200x50", |b| {
        b.iter(|| Buffer::new(black_box(200), black_box(50)));
    });
}

fn bench_buffer_clear(c: &mut Criterion) {
    let mut buf = Buffer::new(200, 50);
    c.bench_function("buffer_clear_200x50", |b| {
        b.iter(|| buf.clear());
    });
}

fn bench_buffer_clear_with_bg(c: &mut Criterion) {
    let mut buf = Buffer::new(200, 50);
    c.bench_function("buffer_clear_with_bg_200x50", |b| {
        b.iter(|| buf.clear_with_bg(Color::Rgb(30, 30, 30)));
    });
}

fn bench_set_char_ascii(c: &mut Criterion) {
    let mut buf = Buffer::new(80, 24);
    c.bench_function("set_char_ascii_80x24", |b| {
        b.iter(|| {
            for y in 0..24 {
                for x in 0..80 {
                    buf.set_char(x, y, 'A', Color::Reset, Color::Reset, Modifier::empty());
                }
            }
        });
    });
}

fn bench_set_char_wide(c: &mut Criterion) {
    let mut buf = Buffer::new(80, 24);
    c.bench_function("set_char_wide_80x24", |b| {
        b.iter(|| {
            for y in 0..24 {
                for x in (0..80).step_by(2) {
                    buf.set_char(x, y, '\u{4e2d}', Color::Reset, Color::Reset, Modifier::empty());
                }
            }
        });
    });
}

fn bench_set_string(c: &mut Criterion) {
    let mut buf = Buffer::new(200, 50);
    let line = "The quick brown fox jumps over the lazy dog. ";
    c.bench_function("set_string_200x50", |b| {
        b.iter(|| {
            for y in 0..50 {
                buf.set_string(0, y, black_box(line), Color::Reset, Color::Reset, Modifier::empty());
            }
        });
    });
}

fn bench_fill(c: &mut Criterion) {
    let mut buf = Buffer::new(200, 50);
    let cell = Cell::from_char('#');
    c.bench_function("fill_100x25_region", |b| {
        b.iter(|| {
            buf.fill(Rect::new(10, 5, 100, 25), cell);
        });
    });
}

fn bench_subview_write(c: &mut Criterion) {
    let mut buf = Buffer::new(200, 50);
    let text = "Hello, world! This is a benchmark test string for subview writing.";
    c.bench_function("subview_write_str_clipped", |b| {
        b.iter(|| {
            let mut sv = buf.subview_mut(Rect::new(5, 5, 80, 20));
            for y in 0..20 {
                sv.write_str_clipped(0, y, black_box(text), Color::Reset, Color::Reset);
            }
        });
    });
}

fn bench_buffer_diff(c: &mut Criterion) {
    let mut front = Buffer::new(200, 50);
    let mut back = Buffer::new(200, 50);
    for y in 0..50 {
        for x in 0..200 {
            front.set_char(x, y, '.', Color::Reset, Color::Reset, Modifier::empty());
        }
    }
    for y in 0..50 {
        for x in 0..200 {
            if (x + y) % 7 == 0 {
                back.set_char(x, y, '#', Color::Cyan, Color::Reset, Modifier::BOLD);
            } else {
                back.set_char(x, y, '.', Color::Reset, Color::Reset, Modifier::empty());
            }
        }
    }
    c.bench_function("diff_scan_200x50_sparse", |b| {
        b.iter(|| {
            let mut dirty = 0usize;
            for i in 0..front.cells.len() {
                if front.cells[i] != back.cells[i] {
                    dirty += 1;
                }
            }
            black_box(dirty);
        });
    });
}

fn bench_poison_all(c: &mut Criterion) {
    let mut buf = Buffer::new(200, 50);
    c.bench_function("poison_all_200x50", |b| {
        b.iter(|| buf.poison_all());
    });
}

fn bench_rect_intersection(c: &mut Criterion) {
    let a = Rect::new(10, 10, 100, 50);
    let b = Rect::new(50, 30, 80, 40);
    c.bench_function("rect_intersection", |b_iter| {
        b_iter.iter(|| black_box(a).intersection(black_box(&b)));
    });
}

criterion_group!(
    benches,
    bench_buffer_new,
    bench_buffer_clear,
    bench_buffer_clear_with_bg,
    bench_set_char_ascii,
    bench_set_char_wide,
    bench_set_string,
    bench_fill,
    bench_subview_write,
    bench_buffer_diff,
    bench_poison_all,
    bench_rect_intersection,
);
criterion_main!(benches);
