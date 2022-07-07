```sh
$ cargo install save --version 0.20220708.0
```

```sh
$ save --help
```

```text
save v0.20220708.0
Would you like to SAVE the change?

Commit everything in the current Git repository, no questions asked.

USAGE:
    save [OPTIONS]

OPTIONS:
    -n, --dry-run
            Prepare the commit, but don't actually update any references in Git.
            
            The commit will be written to the Git database, so it is still possible for the user to
            manually add a reference to it.

    -q, --quiet
            Decrease log verbosity. May be repeated to decrease verbosity further

    -v, --verbose
            Increase log verbosity. May be repeated to increase verbosity further

    -h, --help
            Print help information

    -V, --version
            Print version information

COMMIT OPTIONS:
    -m, --message <MESSAGE>
            The commit message.
            
            [default: a short string based on the commit's tree hash and ancestry graph]
            
            [env: SAVE_COMMIT_MESSAGE=]

    -x, --prefix <PREFIX_HEX>
            The required commit hash or prefix, in hex.
            
            [default: the first four hex digits of the commit's tree hash]
            
            [env: SAVE_COMMIT_PREFIX=]

TREE OPTIONS:
    -a, --all
            Commit all files in the repository. This is the default.
            
            The commit will fail if there are no changes, unless `--allow-empty` is set.

    -s, --staged
            Commit only files that have been explicitly staged with `git add`.
            
            This is like the default behaviour of `git commit`. The commit will fail if there are no
            staged changes unless `--allow-empty` is set.

        --tree <TREE>
            Include the specified tree object in the commit, without looking at or modifying the
            index or working tree

    -e, --empty
            Don't include any file changes in the commit.
            
            This commit will have the same tree hash as its parent.

        --allow-empty
            Create the commit even if it contains no changes

SIGNATURE OPTIONS:
    -t, --timestamp <TIMESTAMP>
            Override the system clock timestamp value
            
            [env: SAVE_TIMESTAMP=]

    -0, --timeless
            Use the next available timestamp after the parent commit's timestamps, regardless of the
            actual current clock time. Assuming there is a parent commit, this is equivalent to
            `--timestamp=0`. If we're creating an initial commit (with no parents), this uses the
            next available timestamp after the current time (or value provided to `--timestamp`)
            rounded down to the closest multiple of `0x1000000` (a period of ~6 months).
            
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

GRAPH OPTIONS:
    -u, --squash
            Squashes these changes into the first parent. May be repeated multiple times to squash
            multiple generations
            
            [aliases: amend]

        --squash-to <SQUASH_TO_REF>
            Squashes all changes from this commit up to the specified ancestor commit.
            
            This will fail if the specified commit isn't actually an ancestor.

    -p, --add-parent <PARENT_REF>
            Adds another parent to the new commit. May be repeated to add multiple parents, though
            duplicated parents will are ignored

        --remove-parent <REMOVED_PARENT_REF>
            Removes a parent from the new commit. May be repeated to remove multiple parents. If the
            parent is not present, this will fail with an error

LINKS:
    https://docs.rs/save/v0.20220708.0
    https://crates.io/crates/save/v0.20220708.0
```
