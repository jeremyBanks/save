use {
    criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput},
    eyre::Context,
    git2::{ObjectType, Oid, Repository},
    rand::{RngCore, SeedableRng},
    rand_pcg::Pcg64,
    rayon::iter::{IntoParallelRefIterator, ParallelIterator},
    save::git2::*,
    std::{path::PathBuf, time::Duration},
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

    c.throughput(Throughput::Elements(arrays.len().try_into().unwrap()));

    c.bench_with_input("from_bytes (git2)", &arrays, |b, arrays| {
        b.iter(|| {
            for array in arrays.iter() {
                Oid::from_bytes(array).unwrap();
            }
        })
    });

    c.bench_with_input("from_array (save)", &arrays, |b, arrays| {
        b.iter(|| {
            for array in arrays.iter() {
                black_box(Oid::from_array(black_box(*array)));
            }
        })
    });
}

fn bench_hash_git_object(c: &mut Criterion) {
    let mut c = c.benchmark_group("hashing git commits");
    c.measurement_time(12 * Duration::from_secs(1));

    for (elements, average_size, label) in [
        (65_536, 512, "64K 512B objects"),
        (1_024, 16_384, "1K 16KB objects"),
    ] {
        let mut rng = Pcg64::seed_from_u64(0);
        let mut bodies = vec![];
        let mut total_len = 0;
        for _ in 0..elements {
            let len = (rng.next_u64() % (average_size * 2)).try_into().unwrap();
            total_len += len;
            let mut body = vec![0u8; len];
            rng.fill_bytes(&mut body);
            bodies.push(body);
        }

        c.throughput(Throughput::Bytes(total_len.try_into().unwrap()));

        c.bench_with_input(
            format!("{label} single-threaded hash_object (git2)"),
            &bodies,
            |b, bodies| {
                b.iter(|| {
                    for body in bodies.iter() {
                        black_box(Oid::hash_object(ObjectType::Commit, body)).unwrap();
                    }
                })
            },
        );

        c.bench_with_input(
            format!("{label} single-threaded for_object (save)"),
            &bodies,
            |b, bodies| {
                b.iter(|| {
                    for body in bodies.iter() {
                        black_box(Oid::for_object("commit", body));
                    }
                })
            },
        );

        c.bench_with_input(
            format!("{label} rayon-parallel hash_object (git2)"),
            &bodies,
            |b, bodies| {
                b.iter(|| {
                    bodies
                        .par_iter()
                        .map(|body| {
                            Oid::hash_object(ObjectType::Commit, body).unwrap();
                        })
                        .max()
                })
            },
        );

        c.bench_with_input(
            format!("{label} rayon-parallel for_object (save)"),
            &bodies,
            |b, bodies| {
                b.iter(|| {
                    bodies
                        .par_iter()
                        .map(|body| Oid::for_object("commit", body))
                        .max()
                })
            },
        );
    }
}

fn bench_generation_number(c: &mut Criterion) {
    let mut c = c.benchmark_group("measuring generation numbers");

    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let test_repos_root = project_root.join("test_repos");

    for name in ["deno", "git", "rust", "typescript"] {
        let repo_path = test_repos_root.join(name);
        let repo = Repository::open(&repo_path)
            .wrap_err("you may need to run /scripts/test-repos")
            .unwrap();

        let commit = repo.head().unwrap().peel_to_commit().unwrap();

        let generation_number = commit.generation_number();
        assert_eq!(generation_number, commit.generation_number_via_petgraph());

        c.bench_with_input(format!("{name}/clunky graph"), &commit, |b, commit| {
            b.iter(|| commit.generation_number());
        });

        c.bench_with_input(format!("{name}/clunky petgraph"), &commit, |b, commit| {
            b.iter(|| commit.generation_number_via_petgraph());
        });
    }
}
