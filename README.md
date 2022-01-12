```sh
$ cargo install save
```

```sh
$ save --help
```

```text
save 0.6.0-dev
Would you like to SAVE the change?

Commit everything in the current Git repository, no questions asked.

USAGE:
    save [OPTIONS]

OPTIONS:
        --email <EMAIL>
            The email to use for the commit's author and committer.
            
            [default: email from git, or else from parent commit, or
            else "save"]

    -h, --help
            Print help information

    -m, --message <MESSAGE>
            Commit message to use.
            
            [default: generated from generation number, tree hash, and
            parents]

    -n, --dry-run
            Prepare the commit, but don't actually save anything to
            disk

        --name <NAME>
            The name to use for the commit's author and committer.
            
            [default: name from git, or else from parent commit, or
            else "save"]

    -q, --quiet
            Decrease log verbosity. May be used multiple times

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
            included in Co-Authored-By footers.
            
            [default: 0]

    -t, --step <STEP_SECONDS>
            Seconds of timestamp allocated for each commit to search.
            
            The number of possibilities searched is the half the
            square of this value.
            
            [default: 128]

    -v, --verbose
            Increase log verbosity. May be used multiple times

    -V, --version
            Print version information

    -w, --now <NOW_SECONDS>
            The time is NOW.
            
            [default: the time is ACTUALLY now]

    -x, --hash <HASH_HEX>
            The target commit hash or prefix, in hex.
            
            [default: the commit's tree hash]

    -y, --yes
            Proceed in spite of any warnings
```
