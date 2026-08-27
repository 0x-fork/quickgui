use std::{hint::black_box, sync::Arc};

use criterion::{Criterion, criterion_group, criterion_main};
use quickgui::{
    Color, ListOffset, ListState, Rect, Scene, TextId, TextRun, TextStyle, VirtualList, div,
};

fn virtual_list_benchmarks(c: &mut Criterion) {
    let mut list = VirtualList::new(100_000, 28.0).with_overscan(2);
    list.set_viewport_height(720.0);
    list.scroll_to(1_234_567.0);

    c.bench_function("100k list visible range", |b| {
        b.iter(|| black_box(black_box(&list).visible_rows()))
    });

    let labels: Vec<Arc<str>> = (0..100_000)
        .map(|index| Arc::from(format!("Row {index:06} — retained and virtualized")))
        .collect();
    let mut scene = Scene::new();
    let style = TextStyle::new(14.0, Color::rgb8(224, 226, 231));
    c.bench_function("100k list build visible scene", |b| {
        b.iter(|| {
            scene.clear(Color::BLACK);
            let list = black_box(&list);
            for index in list.visible_rows().range {
                let rect = list.row_rect(index, 0.0, 0.0, 900.0);
                scene.push_text(TextRun::new(
                    TextId::new(index as u64),
                    labels[index].clone(),
                    Rect::new(rect.x + 12.0, rect.y + 4.0, rect.width - 24.0, 20.0),
                    style.clone(),
                ));
            }
            black_box(scene.text_runs().len())
        })
    });

    let variable = ListState::new(100_000, 56.0).with_overscan(3);
    variable.set_viewport_size(900.0, 720.0);
    variable.scroll_to(ListOffset {
        item_ix: 42_000,
        offset_in_item: 17.0,
    });
    c.bench_function("100k variable list visible range", |b| {
        b.iter(|| black_box(black_box(&variable).visible_rows()))
    });
    c.bench_function("100k variable list build visible rows", |b| {
        b.iter(|| {
            let range = black_box(&variable).visible_rows().range;
            let rows = variable.render_rows(range, |index| {
                div().h(if index.is_multiple_of(3) { 72.0 } else { 44.0 })
            });
            black_box(rows)
        })
    });
}

criterion_group!(benches, virtual_list_benchmarks);
criterion_main!(benches);
