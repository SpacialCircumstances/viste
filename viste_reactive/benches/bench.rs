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

pub fn stream_benchmark(c: &mut Criterion) {
    c.bench_function("stream many", |b| {
        b.iter(|| {
            let world = World::new();
            let (mut portal_setters, mut portal_signals) = (0..25).map(|_| portal(&world)).fold(
                (Vec::new(), Vec::new()),
                |(mut setters, mut signals), (setter, signal)| {
                    setters.push(setter);
                    signals.push(signal);
                    (setters, signals)
                },
            );
            let s = many(&world, portal_signals);
            let c = s.count();

            for _ in 0..100 {
                for setter in &portal_setters {
                    for i in 0..10 {
                        (setter)(black_box(i));
                    }
                }

                let r = c.create_reader();
                let res = c.compute(r).unwrap_changed();
                c.destroy_reader(r);
            }
        })
    });
}

criterion_group!(benches, simple_benchmark, stream_benchmark);
criterion_main!(benches);
