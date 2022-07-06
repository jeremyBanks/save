#[track_caller]
pub fn assert_debug_eq(expected: impl expect_test::ExpectedData, actual: impl ::core::fmt::Debug) {
    ::expect_test::expect(expected).assert_eq(&format!("{actual:?}"));
}

#[track_caller]
pub fn assert_at(path: &'static str, actual: impl ::core::fmt::Display) {
    ::expect_test::expect_file(path).assert_eq(&format!("{actual}"))
}
