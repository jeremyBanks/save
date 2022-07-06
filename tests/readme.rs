use ::save::testing::assert_at;

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
    assert_at("../README.md", &actual);
}
