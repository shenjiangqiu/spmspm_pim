use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn simpediv(c: &mut Criterion) {
    c.bench_function("fib 20 old", |b| b.iter(|| black_box(1000) / black_box(16)));
    c.bench_function("fib 20 new", |b| b.iter(|| black_box(1000) >> black_box(4)));
}

criterion_group!(benches, simpediv);
criterion_main!(benches);
