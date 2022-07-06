```sh
$ cargo install save --version 0.20220708.0
```

```sh
$ save --help
```

```text
save 0.20220708.0
Would you like to SAVE the change?

Commit everything in the current Git repository, no questions asked.

USAGE:
    save [OPTIONS]

OPTIONS:
    -m, --message <MESSAGE>
            The commit message.
            
            [default: a short string based on the commit's tree hash and ancestry graph]
            
            [env: SAVE_COMMIT_MESSAGE=]

    -a, --all
            Commit all files in the repository. This is the default.
            
            The commit will fail if there are no changes.

    -s, --staged
            Commit only files that have been explicitly staged with `git add`.
            
            This is like the default behaviour of `git commit`. The commit will fail if there are no
            staged changes.

    -e, --empty
            Don't include any file changes in the commit.
            
            This commit will have the same tree hash as its parent.

    -n, --dry-run
            Prepare the commit, but don't actually update any references in Git

    -x, --prefix <PREFIX_HEX>
            The required commit hash or prefix, in hex.
            
            [default: the first four hex digits of the commit's tree hash]
            
            [env: SAVE_COMMIT_PREFIX=]

    -t, --timestamp <TIMESTAMP>
            Override the system clock timestamp value
            
            [env: SAVE_TIMESTAMP=]

    -0, --timeless
            Use the next available timestamp after the previous commit, regardless of the current
            timestamp.
            
            If there is no previous commit, this uses the next available timestamp after the current
            time (or value provided to `--timestamp`) rounded down to the closest multiple of
            `0x1000000` (a period of ~6 months).
            
            This can be used to help produce deterministic timestamps and commit IDs for
            reproducible builds.
            
            [env: SAVE_TIMELESS=]

        --name <NAME>
            The name to use for the commit's author and committer.
            
            [default: name from git, or else from parent commit, or else "user"]
            
            [env: GIT_AUTHOR_NAME=]

        --email <EMAIL>
            The email to use for the commit's author and committer.
            
            [default: email from git, or else from parent commit, or else "user@localhost"]
            
            [env: GIT_AUTHOR_EMAIL=]

        --squash
            Squashes these changes into the first parent. May be repeated multiple times to squash
            multiple ancestors
            
            [aliases: amend]

        --add-parent <ADDED_PARENT_REF>
            Adds another parent to the new commit. May be repeated to add multiple parents

        --rm-parent <REMOVED_PARENT_REF>
            Removes a parent from the new commit. May be repeated to remove multiple parents

    -q, --quiet
            Decrease log verbosity. May be repeated to decrease verbosity further

    -v, --verbose
            Increase log verbosity. May be repeated to increase verbosity further

    -h, --help
            Print help information

    -V, --version
            Print version information

LINKS:
    https://docs.rs/save/0.20220708.0
    https://crates.io/crates/save/0.20220708.0
```
