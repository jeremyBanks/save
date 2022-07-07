use ::save::testing::assert_at;

#[test]
fn readme() {
    let mut actual = String::new();
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
    let version = env!("CARGO_PKG_VERSION");
    actual.push_str(&format!(
        "```sh
$ cargo install save --version {version}
```

```sh
$ save --help
```

```text
"
    ));
    actual.push_str(&String::from_utf8(long.stdout.clone()).unwrap());
    actual.push_str("```\n");
    assert_at("../README.md", &actual);
    assert_at("./usage-long.txt", &String::from_utf8(long.stdout).unwrap());
    assert_at(
        "./usage-short.txt",
        &String::from_utf8(short.stdout).unwrap(),
    );
}
