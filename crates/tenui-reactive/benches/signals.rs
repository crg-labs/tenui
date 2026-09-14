use std::cell::Cell;
use std::rc::Rc;

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use tenui_reactive::{Signal, batch};

fn bench_signal_set(c: &mut Criterion) {
    let sig = Signal::new(0i32);
    c.bench_function("signal_set_1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                sig.set(black_box(i));
            }
        });
    });
}

fn bench_signal_get(c: &mut Criterion) {
    let sig = Signal::new(42i32);
    c.bench_function("signal_get_1000", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                black_box(sig.get());
            }
        });
    });
}

fn bench_signal_with_listener(c: &mut Criterion) {
    let sig = Signal::new(0i32);
    let count = Rc::new(Cell::new(0u64));
    let count_c = Rc::clone(&count);
    sig.subscribe(move || {
        count_c.set(count_c.get() + 1);
    });
    c.bench_function("signal_set_with_listener_1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                sig.set(black_box(i));
            }
        });
    });
}

fn bench_signal_fanout(c: &mut Criterion) {
    let sig = Signal::new(0i32);
    let total = Rc::new(Cell::new(0u64));
    for _ in 0..10 {
        let t = Rc::clone(&total);
        sig.subscribe(move || {
            t.set(t.get() + 1);
        });
    }
    c.bench_function("signal_fanout_10_listeners_x100", |b| {
        b.iter(|| {
            for i in 0..100 {
                sig.set(black_box(i));
            }
        });
    });
}

fn bench_batch_100(c: &mut Criterion) {
    let sig = Signal::new(0i32);
    let count = Rc::new(Cell::new(0u64));
    let count_c = Rc::clone(&count);
    sig.subscribe(move || {
        count_c.set(count_c.get() + 1);
    });
    c.bench_function("batch_100_sets", |b| {
        b.iter(|| {
            batch(|| {
                for i in 0..100 {
                    sig.set(black_box(i));
                }
            });
        });
    });
}

fn bench_batch_vs_unbatched(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_vs_unbatched_100");

    let sig = Signal::new(0i32);
    let count = Rc::new(Cell::new(0u64));
    let count_c = Rc::clone(&count);
    sig.subscribe(move || {
        count_c.set(count_c.get() + 1);
    });

    group.bench_function("unbatched", |b| {
        b.iter(|| {
            for i in 0..100 {
                sig.set(black_box(i));
            }
        });
    });

    group.bench_function("batched", |b| {
        b.iter(|| {
            batch(|| {
                for i in 0..100 {
                    sig.set(black_box(i));
                }
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_signal_set,
    bench_signal_get,
    bench_signal_with_listener,
    bench_signal_fanout,
    bench_batch_100,
    bench_batch_vs_unbatched,
);
criterion_main!(benches);
