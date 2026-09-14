use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use taffy::prelude::*;
use tenui_core::{Buffer, Color, Modifier};
use tenui_layout::Ui;

fn build_flat_column(ui: &mut Ui<'_>, n: usize) {
    for _ in 0..n {
        ui.leaf(
            Style {
                size: Size {
                    width: percent(1.0),
                    height: length(1.0),
                },
                ..Default::default()
            },
            |sv| {
                sv.set_string(0, 0, "x", Color::Reset, Color::Reset, Modifier::empty());
            },
        );
    }
}

fn bench_layout_100(c: &mut Criterion) {
    c.bench_function("layout_100_leaves", |b| {
        b.iter(|| {
            let mut buf = Buffer::new(80, 200);
            let mut ui = Ui::new();
            ui.column(Style::default(), |ui| {
                build_flat_column(ui, black_box(100));
            });
            ui.render_to_buffer(&mut buf);
        });
    });
}

fn bench_layout_1000(c: &mut Criterion) {
    c.bench_function("layout_1000_leaves", |b| {
        b.iter(|| {
            let mut buf = Buffer::new(80, 2000);
            let mut ui = Ui::new();
            ui.column(Style::default(), |ui| {
                build_flat_column(ui, black_box(1000));
            });
            ui.render_to_buffer(&mut buf);
        });
    });
}

fn bench_nested_layout(c: &mut Criterion) {
    c.bench_function("layout_nested_10x10", |b| {
        b.iter(|| {
            let mut buf = Buffer::new(200, 200);
            let mut ui = Ui::new();
            ui.column(Style::default(), |ui| {
                for _ in 0..10 {
                    ui.row(
                        Style {
                            size: Size {
                                width: percent(1.0),
                                height: length(20.0),
                            },
                            ..Default::default()
                        },
                        |ui| {
                            for _ in 0..10 {
                                ui.leaf(
                                    Style {
                                        size: Size {
                                            width: percent(0.1),
                                            height: percent(1.0),
                                        },
                                        ..Default::default()
                                    },
                                    |sv| {
                                        sv.set_string(0, 0, ".", Color::Reset, Color::Reset, Modifier::empty());
                                    },
                                );
                            }
                        },
                    );
                }
            });
            ui.render_to_buffer(&mut buf);
        });
    });
}

fn bench_grid_layout(c: &mut Criterion) {
    c.bench_function("layout_grid_8x8", |b| {
        b.iter(|| {
            let mut buf = Buffer::new(80, 80);
            let mut ui = Ui::new();
            ui.grid(
                Style {
                    grid_template_columns: vec![fr(1.0); 8],
                    grid_template_rows: vec![fr(1.0); 8],
                    ..Default::default()
                },
                |ui| {
                    for _ in 0..64 {
                        ui.leaf(Style::default(), |sv| {
                            sv.set_string(0, 0, "#", Color::Reset, Color::Reset, Modifier::empty());
                        });
                    }
                },
            );
            ui.render_to_buffer(&mut buf);
        });
    });
}

criterion_group!(
    benches,
    bench_layout_100,
    bench_layout_1000,
    bench_nested_layout,
    bench_grid_layout
);
criterion_main!(benches);
