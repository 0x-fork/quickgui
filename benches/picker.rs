use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use quickgui::{PickerItem, PickerState};

fn picker_benchmarks(c: &mut Criterion) {
    let items = (0..25_000).map(|index| {
        PickerItem::new(format!("Command {index:05}"), index)
            .detail(format!("Run workspace operation {index}"))
            .keywords(format!("workspace quick action group{}", index % 32))
    });
    let mut picker = PickerState::new(items).expect("benchmark registry is bounded");
    let mut alternate = false;

    c.bench_function("25k picker fuzzy query", |b| {
        b.iter(|| {
            alternate = !alternate;
            picker.set_query(black_box(if alternate {
                "cmd249"
            } else {
                "quick group17"
            }));
            black_box((picker.result_count(), picker.total_match_count()))
        })
    });
}

criterion_group!(benches, picker_benchmarks);
criterion_main!(benches);
