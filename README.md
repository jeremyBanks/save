```sh
$ cargo install save
```

```sh
$ save --help
```

```text
save 0.7.0-dev.0
Would you like to SAVE the change?

Commit everything in the current Git repository, no questions asked.

USAGE:
    save [OPTIONS]

OPTIONS:
    -m, --message <MESSAGE>
            Use this commit message, instead of the default.
            
            [default: generated from generation number, tree hash, and
            parents]

    -a, --all
            Commit all files in the repository. This is the default

    -e, --empty
            Don't include any file changes in the commit.
            
            This commit will have the same tree hash as its parent.

    -s, --squash <SQUASH_COMMITS>
            Squash/amend previous commit(s), instead of adding a new
            one.
            
            By default, `--squash` will behave like `git commit
            --amend`, only replacing the most recent commit. However,
            specifying a larger number such as `--squash=2` will
            squash that many recent first-parents (and any current
            changes) into a single commit. If any of those commits are
            merges, any non-squashed parents will be added as parents
            of the squashed commit. Any additional authors will be
            included in Co-Authored-By footers. Commit messages will
            be discarded.
            
            [default: 0]

    -x, --prefix <PREFIX_HEX>
            The target commit hash or prefix, in hex.
            
            [default: the first four hex digits of the commit's tree
            hash]

    -t, --timestamp <TIMESTAMP>
            Override the system clock timestamp with a custom one

    -0, --timeless
            Use the next available timestamp after the previous
            commit, regardless of the current timestamp.
            
            If there is no previous commit, this uses the next
            available timestamp after the current time (or value
            provided to `--now`) rounded down to the closest multiple
            of `0x1000000` (a period of ~6 months).
            
            This can be used to help produce deterministic timestamps
            and commit IDs for reproducible builds.

        --name <NAME>
            The name to use for the commit's author and committer.
            
            [default: name from git, or else from parent commit, or
            else "dev"]

        --email <EMAIL>
            The email to use for the commit's author and committer.
            
            [default: email from git, or else from parent commit, or
            else "dev@localhost"]

    -n, --dry-run
            Prepare the commit, but don't actually update any
            references in Git

    -q, --quiet
            Decrease log verbosity. May be used multiple times

    -v, --verbose
            Increase log verbosity. May be used multiple times

    -h, --help
            Print help information

    -V, --version
            Print version information

[38;5;11mLINKS:[39m
    https://docs.rs/save/%3C%3D0.7.0-dev.0
    https://crates.io/crates/save
    https://github.com/jeremyBanks/save
```
