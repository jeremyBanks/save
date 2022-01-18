#![feature(const_eval_limit)]
#![const_eval_limit = "0"]

pub use {
    crossterm::style::{style, Color, Stylize},
    iai::black_box,
};
use {
    git2::{ObjectType, Oid},
    save::git2::OidExt,
};

#[macro_export]
macro_rules! main {
    ($($id:ident),*) => {
        #[cfg(all(unix, not(target_os = "macos")))]
        iai::main!($($id),*);

        #[cfg(not(all(unix, not(target_os = "macos"))))]
        fn main() {
            unsupported_main()
        }

        #[allow(unused)]
        fn unsupported_main() {
            let warning = style("warning").with(Color::Yellow);
            let skipping = style("    Skipping").with(Color::Green);
            eprintln!("{warning}: skipping `iai` benches because valgrind is not supported on this platform");
            eprintln!("{skipping} iai ({})", vec![$(stringify!($id)),*].join(", "));
            let _unused = [$($id),*];
        }
    }
}
fn bench_hash_object_git2() {
    black_box(Oid::hash_object(
        ObjectType::Commit,
        black_box(SMALL_BODY.as_ref()),
    ))
    .ok();
}

fn bench_hash_object_save() {
    black_box(Oid::for_object("commit", black_box(SMALL_BODY.as_ref())));
}

fn bench_hash_object_git2_large() {
    black_box(Oid::hash_object(
        ObjectType::Commit,
        black_box(LARGE_BODY.as_ref()),
    ))
    .ok();
}

fn bench_hash_object_save_large() {
    black_box(Oid::for_object("commit", black_box(LARGE_BODY.as_ref())));
}

main!(
    bench_hash_object_git2,
    bench_hash_object_save,
    bench_hash_object_git2_large,
    bench_hash_object_save_large
);

const SMALL_BODY: [u8; 512] = assorted_bytes();
const LARGE_BODY: [u8; 1_048_576] = assorted_bytes();

const fn assorted_bytes<const LENGTH: usize>() -> [u8; LENGTH] {
    let mut bytes = [0u8; LENGTH];
    let mut i = 0;
    while i < LENGTH {
        bytes[i] = (i as u8) % 241;
        i += 1;
    }

    bytes
}
