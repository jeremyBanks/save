use ::{
    core::{mem, ops::Range, panic, fmt::{self, Debug}},
    once_cell::sync::{Lazy, OnceCell},
    std::{
        collections::HashMap,
        env, fs,
        path::{Path, PathBuf},
        sync::Mutex,
    },
};

//
//
#[track_caller]
fn assert_eq<Literal: self::Literal>(expected: Literal, actual: Literal) {
    assert!(Expected::from_caller() == actual)
    assert_eq!(expected, actual);
}

pub trait Literal: Clone + Debug + Copy + PartialEq {}

impl Literal for &str {}
impl Literal for &[u8] {}
impl Literal for bool {}
impl Literal for char {}
impl Literal for u8 {}
impl Literal for u16 {}
impl Literal for u32 {}
impl Literal for u64 {}
impl Literal for u128 {}
impl Literal for usize {}
impl Literal for i8 {}
impl Literal for i16 {}
impl Literal for i32 {}
impl Literal for i64 {}
impl Literal for i128 {}
impl Literal for isize {}
impl Literal for f32 {}
impl Literal for f64 {}

#[derive(Clone, Debug)]
pub struct Expected<Literal: self::Literal> {
    pub value: Literal,
    pub by: PathBuf,
    pub at: ExpectedLocation,
}

impl<Literal: self::Literal> PartialEq<Literal> for Expected<Literal> {
    fn eq(&self, other: &Literal) -> bool {
        if self.value == *other {
            true
        } else {
            // inequality! record this if we're in replacement mode
            false
        }
    }
}

#[derive(Clone, Debug)]
pub enum ExpectedLocation {
    InlineLiteral {
        line: usize,
        column: usize,
    },
    ExternalFile {
        path: PathBuf,
    },
}

#[track_caller]
pub fn assert_debug_eq(expected: impl expect_test::ExpectedData, actual: impl ::core::fmt::Debug) {
    expect(expected).assert_eq(&format!("{actual:?}"));
}

#[track_caller]
pub fn assert_at(path: &'static str, actual: impl ::core::fmt::Display) {
    expect_file(path).assert_eq(&format!("{actual}"));
}

fn update_expect() -> bool {
    env::var("SAVE_EXPECTATIONS").is_ok() || env::var("UPDATE_EXPECT").is_ok()
}



#[track_caller]
pub fn expect(data: &'static str) -> Expect {
    let location = std::panic::Location::caller();
    Expect {
        position: Position {
            file: location.file(),
            line: location.line(),
            column: location.column(),
        },
        data: data.str(),
        indent: true,
    }
}

#[macro_export]
macro_rules! expect_file {
    [$path:expr] => {$crate::ExpectFile {
        path: std::path::PathBuf::from($path),
        position: file!(),
    }};
}


pub fn expect_file(path: impl Into<PathBuf>) -> ExpectFile {
    ExpectFile {
        path: path.into(),
        position: std::panic::Location::caller().file(),
    }
}

#[derive(Debug)]
pub struct Expect {
    #[doc(hidden)]
    pub position: Position,
    #[doc(hidden)]
    pub data: &'static str,
    #[doc(hidden)]
    pub indent: bool,
}

#[derive(Debug)]
pub struct ExpectFile {
    #[doc(hidden)]
    pub path: PathBuf,
    #[doc(hidden)]
    pub position: &'static str,
}

#[derive(Debug)]
pub struct Position {
    #[doc(hidden)]
    pub file: &'static str,
    #[doc(hidden)]
    pub line: u32,
    #[doc(hidden)]
    pub column: u32,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

#[derive(Clone, Copy)]
enum StrLitKind {
    Normal,
    Raw(usize),
}

impl StrLitKind {
    fn write_start(self, w: &mut impl std::fmt::Write) -> std::fmt::Result {
        match self {
            Self::Normal => write!(w, "\""),
            Self::Raw(n) => {
                write!(w, "r")?;
                for _ in 0..n {
                    write!(w, "#")?;
                }
                write!(w, "\"")
            },
        }
    }

    fn write_end(self, w: &mut impl std::fmt::Write) -> std::fmt::Result {
        match self {
            Self::Normal => write!(w, "\""),
            Self::Raw(n) => {
                write!(w, "\"")?;
                for _ in 0..n {
                    write!(w, "#")?;
                }
                Ok(())
            },
        }
    }
}

impl Expect {
    pub fn assert_eq(&self, actual: &str) {
        let trimmed = self.trimmed();
        if trimmed == actual {
            return;
        }
        Runtime::fail_expect(self, &trimmed, actual);
    }

    pub fn assert_debug_eq(&self, actual: &impl fmt::Debug) {
        let actual = format!("{:#?}\n", actual);
        self.assert_eq(&actual)
    }

    pub fn indent(&mut self, yes: bool) {
        self.indent = yes;
    }

    pub fn data(&self) -> &str {
        self.data
    }

    fn trimmed(&self) -> String {
        if !self.data.contains('\n') {
            return self.data.to_string();
        }
        trim_indent(self.data)
    }

    fn locate(&self, file: &str) -> Location {
        let mut target_line = None;
        let mut line_start = 0;
        for (i, line) in lines_with_ends(file).enumerate() {
            if i == self.position.line as usize - 1 {
                // `column` points to the first character of the macro invocation/function call:
                //
                //    expect![[r#""#]]    expect![""]    expect("")   expect([""])
                //    ^       ^           ^       ^      ^      ^     ^       ^
                //  column   offset
                //
                // we seek until we find the first character of the string literal, if present.
                let byte_offset = line
                    .char_indices()
                    .skip((self.position.column - 1).try_into().unwrap())
                    .skip_while(|&(_, c)| !matches!(c, '[' | '(' | '{'))
                    // .skip_while(|&(_, c)| matches!(c, '[' | '(' | '{') || c.is_whitespace())
                    .skip(1)
                    .next()
                    .expect("Failed to parse macro invocation")
                    .0;

                let literal_start = line_start + byte_offset;
                let indent = line.chars().take_while(|&it| it == ' ').count();
                target_line = Some((literal_start, indent));
                break;
            }
            line_start += line.len();
        }
        let (literal_start, line_indent) = target_line.unwrap();

        let lit_to_eof = &file[literal_start..];
        let lit_to_eof_trimmed = lit_to_eof.trim_start();

        let literal_start = literal_start + (lit_to_eof.len() - lit_to_eof_trimmed.len());

        let literal_len =
            locate_end(lit_to_eof_trimmed).expect("Couldn't find closing delimiter for `expect`.");
        let literal_range = literal_start..literal_start + literal_len;
        Location {
            line_indent,
            literal_range,
        }
    }
}

fn locate_end(arg_start_to_eof: &str) -> Option<usize> {
    let c = arg_start_to_eof.chars().next()?;
    if c.is_whitespace() {
        panic!("skip whitespace before calling `locate_end`")
    }
    match c {
        // expect![["..."]] | expect!(["..."])
        '[' | '(' => {
            let end = if c == '[' { ']' } else { ')' };
            let str_start_to_eof = arg_start_to_eof[1..].trim_start();
            if str_start_to_eof.chars().next() == Some(end) {
                return Some(2);
            }
            let str_len = find_str_lit_len(str_start_to_eof)?;
            let str_end_to_eof = &str_start_to_eof[str_len..];
            let closing_brace_offset = str_end_to_eof.find(end)?;
            Some((arg_start_to_eof.len() - str_end_to_eof.len()) + closing_brace_offset + 1)
        },

        // expect![] | expect!{} | expect!()
        ']' | '}' | ')' => Some(0),

        // expect!["..."] | expect![r#"..."#] | expect("...")
        _ => find_str_lit_len(arg_start_to_eof),
    }
}

fn find_str_lit_len(str_lit_to_eof: &str) -> Option<usize> {
    use StrLitKind::*;

    fn try_find_n_hashes(
        s: &mut impl Iterator<Item = char>,
        desired_hashes: usize,
    ) -> Option<(usize, Option<char>)> {
        let mut n = 0;
        loop {
            match s.next()? {
                '#' => n += 1,
                c => return Some((n, Some(c))),
            }

            if n == desired_hashes {
                return Some((n, None));
            }
        }
    }

    let mut s = str_lit_to_eof.chars();
    let kind = match s.next()? {
        '"' => Normal,
        'r' => {
            let (n, c) = try_find_n_hashes(&mut s, usize::MAX)?;
            if c != Some('"') {
                return None;
            }
            Raw(n)
        },
        _ => return None,
    };

    let mut oldc = None;
    loop {
        let c = oldc.take().or_else(|| s.next())?;
        match (c, kind) {
            ('\\', Normal) => {
                let _escaped = s.next()?;
            },
            ('"', Normal) => break,
            ('"', Raw(0)) => break,
            ('"', Raw(n)) => {
                let (seen, c) = try_find_n_hashes(&mut s, n)?;
                if seen == n {
                    break;
                }
                oldc = c;
            },
            _ => {},
        }
    }

    Some(str_lit_to_eof.len() - s.as_str().len())
}

impl ExpectFile {
    pub fn assert_eq(&self, actual: &str) {
        let expected = self.read();
        if actual == expected {
            return;
        }
        Runtime::fail_file(self, &expected, actual);
    }

    pub fn assert_debug_eq(&self, actual: &impl fmt::Debug) {
        let actual = format!("{:#?}\n", actual);
        self.assert_eq(&actual)
    }

    fn read(&self) -> String {
        fs::read_to_string(self.abs_path())
            .unwrap_or_default()
            .replace("\r\n", "\n")
    }

    fn write(&self, contents: &str) {
        fs::write(self.abs_path(), contents).unwrap()
    }

    fn abs_path(&self) -> PathBuf {
        if self.path.is_absolute() {
            self.path.to_owned()
        } else {
            let dir = Path::new(self.position).parent().unwrap();
            to_abs_ws_path(&dir.join(&self.path))
        }
    }
}

#[derive(Default)]
struct Runtime {
    help_printed: bool,
    per_file: HashMap<&'static str, FileRuntime>,
}
static RT: Lazy<Mutex<Runtime>> = Lazy::new(Default::default);

impl Runtime {
    fn fail_expect(expect: &Expect, expected: &str, actual: &str) {
        let mut rt = RT.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if update_expect() {
            println!("\x1b[1m\x1b[92mupdating\x1b[0m: {}", expect.position);
            rt.per_file
                .entry(expect.position.file)
                .or_insert_with(|| FileRuntime::new(expect))
                .update(expect, actual);
            return;
        }
        rt.panic(expect.position.to_string(), expected, actual);
    }

    fn fail_file(expect: &ExpectFile, expected: &str, actual: &str) {
        let mut rt = RT.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if update_expect() {
            println!("\x1b[1m\x1b[92mupdating\x1b[0m: {}", expect.path.display());
            expect.write(actual);
            return;
        }
        rt.panic(expect.path.display().to_string(), expected, actual);
    }

    fn panic(&mut self, position: String, expected: &str, actual: &str) {
        let print_help = !mem::replace(&mut self.help_printed, true);
        let help = if print_help { HELP } else { "" };

        let diff = ::dissimilar::diff(expected, actual);

        println!(
            "\n
\x1b[1m\x1b[91merror\x1b[97m: expect test failed\x1b[0m
   \x1b[1m\x1b[34m-->\x1b[0m {}
{}
\x1b[1mExpect\x1b[0m:
----
{}
----

\x1b[1mActual\x1b[0m:
----
{}
----

\x1b[1mDiff\x1b[0m:
----
{}
----
",
            position,
            help,
            expected,
            actual,
            format_chunks(diff)
        );
        // Use resume_unwind instead of panic!() to prevent a backtrace, which is unnecessary noise.
        panic::resume_unwind(Box::new(()));
    }
}

struct FileRuntime {
    path: PathBuf,
    original_text: String,
    patchwork: Patchwork,
}

impl FileRuntime {
    fn new(expect: &Expect) -> FileRuntime {
        let path = to_abs_ws_path(Path::new(expect.position.file));
        let original_text = fs::read_to_string(&path).unwrap();
        let patchwork = Patchwork::new(original_text.clone());
        FileRuntime {
            path,
            original_text,
            patchwork,
        }
    }

    fn update(&mut self, expect: &Expect, actual: &str) {
        let loc = expect.locate(&self.original_text);
        let desired_indent = if expect.indent {
            Some(loc.line_indent)
        } else {
            None
        };
        let patch = format_patch(desired_indent, actual);
        self.patchwork.patch(loc.literal_range, &patch);
        fs::write(&self.path, &self.patchwork.text).unwrap()
    }
}

#[derive(Debug)]
struct Location {
    line_indent: usize,

    literal_range: Range<usize>,
}

#[derive(Debug)]
struct Patchwork {
    text: String,
    indels: Vec<(Range<usize>, usize)>,
}

impl Patchwork {
    fn new(text: String) -> Patchwork {
        Patchwork {
            text,
            indels: Vec::new(),
        }
    }

    fn patch(&mut self, mut range: Range<usize>, patch: &str) {
        self.indels.push((range.clone(), patch.len()));
        self.indels.sort_by_key(|(delete, _insert)| delete.start);

        let (delete, insert) = self
            .indels
            .iter()
            .take_while(|(delete, _)| delete.start < range.start)
            .map(|(delete, insert)| (delete.end - delete.start, insert))
            .fold((0usize, 0usize), |(x1, y1), (x2, y2)| (x1 + x2, y1 + y2));

        for pos in &mut [&mut range.start, &mut range.end] {
            **pos -= delete;
            **pos += insert;
        }

        self.text.replace_range(range, &patch);
    }
}

fn lit_kind_for_patch(patch: &str) -> StrLitKind {
    let has_dquote = patch.chars().any(|c| c == '"');
    if !has_dquote {
        let has_bslash_or_newline = patch.chars().any(|c| matches!(c, '\\' | '\n'));
        return if has_bslash_or_newline {
            StrLitKind::Raw(1)
        } else {
            StrLitKind::Normal
        };
    }

    // Find the maximum number of hashes that follow a double quote in the string.
    // We need to use one more than that to delimit the string.
    let leading_hashes = |s: &str| s.chars().take_while(|&c| c == '#').count();
    let max_hashes = patch.split('"').map(leading_hashes).max().unwrap();
    StrLitKind::Raw(max_hashes + 1)
}

fn format_patch(desired_indent: Option<usize>, patch: &str) -> String {
    let lit_kind = lit_kind_for_patch(patch);
    let indent = desired_indent.map(|it| " ".repeat(it));
    let is_multiline = patch.contains('\n');

    let mut buf = String::new();
    if matches!(lit_kind, StrLitKind::Raw(_)) {
        buf.push('[');
    }
    lit_kind.write_start(&mut buf).unwrap();
    if is_multiline {
        buf.push('\n');
    }
    let mut final_newline = false;
    for line in lines_with_ends(patch) {
        if is_multiline && !line.trim().is_empty() {
            if let Some(indent) = &indent {
                buf.push_str(indent);
                buf.push_str("    ");
            }
        }
        buf.push_str(line);
        final_newline = line.ends_with('\n');
    }
    if final_newline {
        if let Some(indent) = &indent {
            buf.push_str(indent);
        }
    }
    lit_kind.write_end(&mut buf).unwrap();
    if matches!(lit_kind, StrLitKind::Raw(_)) {
        buf.push(']');
    }
    buf
}

fn to_abs_ws_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_owned();
    }

    static WORKSPACE_ROOT: OnceCell<PathBuf> = OnceCell::new();
    WORKSPACE_ROOT
        .get_or_try_init(|| {
            let my_manifest = env::var("CARGO_MANIFEST_DIR")?;

            // Heuristic, see https://github.com/rust-lang/cargo/issues/3946
            let workspace_root = Path::new(&my_manifest)
                .ancestors()
                .filter(|it| it.join("Cargo.toml").exists())
                .last()
                .unwrap()
                .to_path_buf();

            Ok(workspace_root)
        })
        .unwrap_or_else(|_: env::VarError| {
            panic!(
                "No CARGO_MANIFEST_DIR env var and the path is relative: {}",
                path.display()
            )
        })
        .join(path)
}

fn trim_indent(mut text: &str) -> String {
    if text.starts_with('\n') {
        text = &text[1..];
    }
    let indent = text
        .lines()
        .filter(|it| !it.trim().is_empty())
        .map(|it| it.len() - it.trim_start().len())
        .min()
        .unwrap_or(0);

    lines_with_ends(text)
        .map(|line| {
            if line.len() <= indent {
                line.trim_start_matches(' ')
            } else {
                &line[indent..]
            }
        })
        .collect()
}

fn lines_with_ends(text: &str) -> LinesWithEnds {
    LinesWithEnds { text }
}

struct LinesWithEnds<'a> {
    text: &'a str,
}

impl<'a> Iterator for LinesWithEnds<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.text.is_empty() {
            return None;
        }
        let idx = self.text.find('\n').map_or(self.text.len(), |it| it + 1);
        let (res, next) = self.text.split_at(idx);
        self.text = next;
        Some(res)
    }
}

fn format_chunks(chunks: Vec<::dissimilar::Chunk>) -> String {
    let mut buf = String::new();
    for chunk in chunks {
        let formatted = match chunk {
            ::dissimilar::Chunk::Equal(text) => text.into(),
            ::dissimilar::Chunk::Delete(text) => format!("\x1b[41m{}\x1b[0m", text),
            ::dissimilar::Chunk::Insert(text) => format!("\x1b[42m{}\x1b[0m", text),
        };
        buf.push_str(&formatted);
    }
    buf
}
