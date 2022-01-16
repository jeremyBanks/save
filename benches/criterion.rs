use {
    criterion::{black_box, criterion_group, criterion_main, Criterion},
    git2::{ObjectType, Oid, Repository},
    rand::{RngCore, SeedableRng},
    rand_pcg::Pcg64,
    rayon::iter::{IntoParallelRefIterator, ParallelIterator},
    save::git2::{OidExt, RepositoryExt, *},
    std::{ops::Deref, time::Duration},
};

criterion_main!(benches);
criterion_group! {
    benches,
    bench_oid_from_bytes,
    bench_hash_git_object,
    bench_generation_number,
}

fn bench_oid_from_bytes(c: &mut Criterion) {
    let mut c = c.benchmark_group("transmuting git ids");

    let mut rng = Pcg64::seed_from_u64(0);
    let mut arrays = vec![];
    for _ in 0..8_192 {
        let mut array = [0u8; 20];
        rng.fill_bytes(&mut array);
        arrays.push(array);
    }

    c.bench_function("from_bytes (git2)", |b| {
        b.iter(|| {
            for array in arrays.iter() {
                black_box(Oid::from_bytes(black_box(array))).unwrap();
            }
        })
    });

    c.bench_function("from_array (save)", |b| {
        b.iter(|| {
            for array in arrays.iter() {
                black_box(Oid::from_array(black_box(*array)));
            }
        })
    });
}

fn bench_hash_git_object(c: &mut Criterion) {
    let mut c = c.benchmark_group("hashing git objects");
    c.measurement_time(12 * Duration::from_secs(1));

    let mut rng = Pcg64::seed_from_u64(0);
    let mut bodies = vec![];
    for _ in 0..8_192 {
        let len = (64 + rng.next_u64() % 896).try_into().unwrap();
        let mut body = vec![0u8; len];
        rng.fill_bytes(&mut body);
        bodies.push(body);
    }

    c.bench_function("single-threaded hash_object (git2)", |b| {
        b.iter(|| {
            for body in bodies.iter() {
                black_box(Oid::hash_object(ObjectType::Commit, black_box(body))).unwrap();
            }
        })
    });

    c.bench_function("single-threaded for_object (save)", |b| {
        b.iter(|| {
            for body in bodies.iter() {
                black_box(Oid::for_object("commit", black_box(body)));
            }
        })
    });

    c.bench_function("rayon-parallel hash_object (git2)", |b| {
        b.iter(|| {
            bodies.par_iter().for_each(|body| {
                black_box(Oid::hash_object(ObjectType::Commit, black_box(body))).unwrap();
            });
        })
    });

    c.bench_function("rayon-parallel for_object (save)", |b| {
        b.iter(|| {
            bodies.par_iter().for_each(|body| {
                black_box(Oid::for_object("commit", black_box(body)));
            });
        })
    });
}

fn bench_generation_number(c: &mut Criterion) {
    let mut c = c.benchmark_group("measuring generation numbers");

    let mut repo = Repository::temporary().unwrap();
    // repo.commit(Some("HEAD"), )
    // TODO: run .save() a bunch of times.
    // I guess that should be a RepositoryExt method, eh?

    let commit = repo.head().unwrap().peel_to_commit().unwrap();

    c.bench_function("my clunky graph", |b| {
        b.iter(|| {
            black_box(black_box(&commit).generation_number());
        })
    });

    c.bench_function("my clunky petgraph", |b| {
        b.iter(|| {
            black_box(black_box(&commit).generation_number_via_petgraph());
        })
    });
}
