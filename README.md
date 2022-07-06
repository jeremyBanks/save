```
save 0.20220707.0
Would you like to SAVE the change?

Commit everything in the current Git repository, no questions asked.

USAGE:
    save [OPTIONS]

OPTIONS:
    -m, --message <MESSAGE>
            Use this commit message, instead of the default.
            
            [default: generated from generation number, tree hash, and parents]
            
            [env: SAVE_COMMIT_MESSAGE=]

    -a, --all
            Commit all files in the repository. This is the default

    -e, --empty
            Don't include any file changes in the commit.
            
            This commit will have the same tree hash as its parent.

    -x, --prefix <PREFIX_HEX>
            The required commit hash or prefix, in hex.
            
            [default: the first four hex digits of the commit's tree hash]
            
            [env: SAVE_COMMIT_PREFIX=]

        --name <NAME>
            The name to use for the commit's author and committer.
            
            [default: name from git, or else from parent commit, or else "dev"]
            
            [env: GIT_AUTHOR_NAME=]

        --email <EMAIL>
            The email to use for the commit's author and committer.
            
            [default: email from git, or else from parent commit, or else "dev@localhost"]
            
            [env: GIT_AUTHOR_EMAIL=]

    -n, --dry-run
            Prepare the commit, but don't actually update any references in Git

    -q, --quiet
            Decrease log verbosity. May be used multiple times

    -v, --verbose
            Increase log verbosity. May be used multiple times

    -h, --help
            Print help information

    -V, --version
            Print version information

LINKS:
    https://docs.rs/save
    https://crates.io/crates/save
```
