use {
    crossterm::style::{style, Color, Stylize},
    iai::black_box,
};

#[macro_export]
macro_rules! main {
    ($($id:ident),*) => {
        #[cfg(all(unix, not(target_os = "macos")))]
        iai::main!($($id),*);

        #[cfg(not(all(unix, not(target_os = "macos"))))]
        fn main() {
            let warning = style("warning").with(Color::Yellow);
            let skipping = style("    Skipping").with(Color::Green);
            eprintln!("{warning}: skipping `iai` benches because valgrind is not supported on this platform");
            eprintln!("{skipping} iai ({})", vec![$(stringify!($id)),*].join(", "));
            let _unused = [$($id),*];
        }
    }
}

fn bench_hash_object_git2() {
    black_box(2);
}

fn bench_hash_object_save() {
    black_box(3);
}

main!(bench_hash_object_git2, bench_hash_object_save);
