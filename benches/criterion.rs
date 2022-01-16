use {
    criterion::{black_box, criterion_group, criterion_main, Criterion},
    git2::{ObjectType, Oid},
    rand::{RngCore, SeedableRng},
    rand_pcg::Pcg64,
    rayon::iter::{IntoParallelRefIterator, ParallelIterator},
    save::git2::OidExt,
    std::time::Duration,
};

fn bench_oid_from_bytes(c: &mut Criterion) {
    let mut rng = Pcg64::seed_from_u64(0);
    let mut arrays = vec![];
    for _ in 0..8_192 {
        let mut array = [0u8; 20];
        rng.fill_bytes(&mut array);
        arrays.push(array);
    }

    let mut c = c.benchmark_group("transmute git ids");

    c.bench_function("from_bytes from git2", |b| {
        b.iter(|| {
            for array in arrays.iter() {
                black_box(Oid::from_bytes(black_box(array))).unwrap();
            }
        })
    });

    c.bench_function("from_array from save", |b| {
        b.iter(|| {
            for array in arrays.iter() {
                black_box(Oid::from_array(black_box(*array)));
            }
        })
    });
}

fn bench_hash_git_object(c: &mut Criterion) {
    let mut rng = Pcg64::seed_from_u64(0);
    let mut bodies = vec![];
    for _ in 0..8_192 {
        let len = (64 + rng.next_u64() % 896).try_into().unwrap();
        let mut body = vec![0u8; len];
        rng.fill_bytes(&mut body);
        bodies.push(body);
    }

    let mut c = c.benchmark_group("hashing git objects");

    c.bench_function("single-threaded hash_object from git2", |b| {
        b.iter(|| {
            for body in bodies.iter() {
                black_box(Oid::hash_object(ObjectType::Commit, black_box(body))).unwrap();
            }
        })
    });

    c.bench_function("single-threaded for_object from save", |b| {
        b.iter(|| {
            for body in bodies.iter() {
                black_box(Oid::for_object("commit", black_box(body)));
            }
        })
    });

    c.bench_function("rayon-parallel hash_object from git2", |b| {
        b.iter(|| {
            bodies.par_iter().for_each(|body| {
                black_box(Oid::hash_object(ObjectType::Commit, black_box(body))).unwrap();
            });
        })
    });

    c.bench_function("rayon-parallel for_object from save", |b| {
        b.iter(|| {
            bodies.par_iter().for_each(|body| {
                black_box(Oid::for_object("commit", black_box(body)));
            });
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(64).measurement_time(Duration::from_secs(16));
    targets =
        bench_oid_from_bytes,
        bench_hash_git_object,

}
criterion_main!(benches);
