use criterion::*;

fn bench_hash_object(c: &mut Criterion) {
    c.bench_function("git2::Oid::hash_thing", |b| b.iter(|| (black_box(20))));
    c.bench_function("save::git2::OidExt::hash_bytes", |b| {
        b.iter(|| (black_box(20)))
    });
}

criterion_group!(benches, bench_hash_object);
criterion_main!(benches);
