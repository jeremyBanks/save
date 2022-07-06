use ::save::testing::assert_at;

#[test]
fn readme() {
    let mut actual = String::new();
    let output = std::process::Command::new("cargo")
        .args(["run", "--", "--help"])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap()
        .wait_with_output()
        .unwrap();
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
    actual.push_str(&String::from_utf8(output.stdout).unwrap());
    actual.push_str("```\n");
    assert_at("../README.md", &actual);
}
