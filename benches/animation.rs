use std::{hint::black_box, time::Duration};

use criterion::{Criterion, criterion_group, criterion_main};
use quickgui::{
    Animation, AnimationExt, AnimationPhase, Color, SpringAnimation, SpringConfig, SpringState,
    Transition, div, ease_in_out,
};

fn animation_benchmarks(c: &mut Criterion) {
    let spring = SpringConfig::new(170.0, 26.0, 1.0);
    let state = SpringState {
        position: 0.0,
        velocity: 180.0,
    };

    c.bench_function("analytic spring display step", |b| {
        b.iter(|| {
            black_box(spring.step(black_box(state), black_box(420.0), black_box(1.0 / 120.0)))
        })
    });

    c.bench_function("analytic spring delayed jump", |b| {
        b.iter(|| black_box(spring.step(black_box(state), black_box(420.0), black_box(0.25))))
    });

    c.bench_function("animation phase style projection", |b| {
        b.iter(|| {
            let value = ease_in_out(black_box(0.42));
            let phase = AnimationPhase(value);
            black_box((
                phase.interpolate_clamped(24.0, 420.0),
                phase.interpolate_clamped(Color::rgb8(249, 115, 22), Color::rgb8(34, 197, 94)),
            ))
        })
    });

    c.bench_function("declarative motion construction", |b| {
        b.iter(|| {
            let element = div()
                .with_animation(
                    "bench-time",
                    Animation::new(Duration::from_millis(240)).with_easing(ease_in_out),
                    |element, value| element.w(value * 420.0),
                )
                .with_spring(
                    "bench-spring",
                    SpringAnimation::new(spring).to(420.0),
                    |element, value| element.left(value),
                );
            black_box(element)
        })
    });

    c.bench_function("style transition declaration construction", |b| {
        b.iter(|| {
            black_box(
                div()
                    .bg(Color::rgb8(51, 65, 85))
                    .hover(|style| style.bg(Color::rgb8(79, 70, 229)).rounded(14.0))
                    .transition(Transition::colors(Duration::from_millis(140))),
            )
        })
    });
}

criterion_group!(benches, animation_benchmarks);
criterion_main!(benches);
