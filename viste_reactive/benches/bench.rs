use criterion::{black_box, criterion_group, criterion_main, Criterion};
use viste_reactive::*;

pub fn simple_benchmark(c: &mut Criterion) {
    c.bench_function("set and map", |b| {
        b.iter(|| {
            let world = World::new();
            let (set, s) = mutable(&world, 1);
            let m = s.map(|x| x + 1);
            set(black_box(5));
            let r = m.signal().create_reader();
            let res = m.signal().compute(r);
            m.signal().destroy_reader(r);
            res.unwrap_changed();
        })
    });
}

fn generate_tree<'a>(
    world: &'a World,
    setters: &mut Vec<Box<dyn Fn(i32)>>,
    depth: u32,
    max_depth: u32,
) -> ValueSignal<'a, i32> {
    if depth == max_depth {
        let (set, v) = mutable(&world, 0);
        setters.push(Box::new(set));
        v
    } else {
        let s1 = generate_tree(world, setters, depth + 1, max_depth);
        let s2 = generate_tree(world, setters, depth + 1, max_depth);
        let m1 = s1.map(|i| i + 1);
        let m2 = s2.map(|i| i + 1);
        map2(&m1, &m2, |i1, i2| i1 + i2)
    }
}

pub fn values_benchmark(c: &mut Criterion) {
    let world = World::new();
    let mut setters = Vec::new();
    let root = generate_tree(&world, &mut setters, 0, 14);

    c.bench_function("value propagation in complex graph", |b| {
        b.iter(|| {
            for i in 0..5 {
                for setter in &setters {
                    (setter)(black_box(i))
                }
            }

            let r = root.signal().create_reader();
            let v = root.signal().compute(r).unwrap_changed();
            root.signal().destroy_reader(r);
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

                let r = c.signal().create_reader();
                let res = c.signal().compute(r).unwrap_changed();
                c.signal().destroy_reader(r);
            }
        })
    });
}

criterion_group!(
    benches,
    simple_benchmark,
    values_benchmark,
    stream_benchmark
);
criterion_main!(benches);
