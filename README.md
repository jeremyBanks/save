```sh
$ cargo install save
```

```sh
$ save --help
```

```text
save 0.5.23-dev
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
