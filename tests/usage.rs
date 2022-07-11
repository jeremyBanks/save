use ::save::testing::assert_at;

#[test]
fn readme() {
    let long = std::process::Command::new("cargo")
        .args(["run", "--", "--help"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    let short = std::process::Command::new("cargo")
        .args(["run", "--", "-h"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
    assert!(
        long.status.success(),
        "failed to generate --help for readme"
    );
    assert_at(
        "../README.txt",
        &String::from_utf8(long.stdout.clone()).unwrap(),
    );
    assert_at("./usage-long.txt", &String::from_utf8(long.stdout).unwrap());
    assert_at(
        "./usage-short.txt",
        &String::from_utf8(short.stdout).unwrap(),
    );
}
