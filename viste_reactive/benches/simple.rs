use criterion::{black_box, criterion_group, criterion_main, Criterion};
use viste_reactive::*;

pub fn simple_benchmark(c: &mut Criterion) {
    c.bench_function("set and map", |b| {
        b.iter(|| {
            let world = World::new();
            let (set, s) = mutable(&world, 1);
            let m = s.map(|x| x + 1);
            set(black_box(5));
            let r = m.create_reader();
            let res = m.compute(r);
            m.destroy_reader(r);
            res.unwrap_changed();
        })
    });
}

criterion_group!(benches, simple_benchmark);
criterion_main!(benches);
