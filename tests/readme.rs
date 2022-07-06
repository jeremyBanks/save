use expect_test::expect_file;

#[test]
fn readme() {
    let mut actual = String::new();
    let output = std::process::Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .unwrap();
    actual.push_str("```\n");
    actual.push_str(&String::from_utf8(output.stdout).unwrap());
    actual.push_str("```\n");
    expect_file!("../README.md").assert_eq(&actual);
}
