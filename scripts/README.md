# `/scripts/`

Internal shell scripts for development of this repository:

- `pre-release` is our pre-release hook invoked automatically by
  `cargo-release`.
- `readme` regenerates `/README.md` from the CLI `--help` output.
- `test-repos` clones and updates a standard set of Git repositories into
  `/test_repos` for use by tests or benchmarks.
