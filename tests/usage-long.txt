save v0.20220708.0
Commit everything in the current directory and repository -- no questions asked.

╔══════════════════╗╔════╗
║Would you like to ║║►YES║
║SAVE the changes? ║║ NO ║
╚══════════════════╝╚════╝

USAGE:
    save [OPTIONS]

OPTIONS:
    -q, --quiet
            Decrease log verbosity. May be repeated to decrease verbosity
            further.
            
            [env: RUST_LOG=]

    -v, --verbose
            Increase log verbosity. May be repeated to increase verbosity
            further.
            
            [env: RUST_LOG=]

    -h, --help
            Print help information

    -V, --version
            Print version information

CONTENT OPTIONS:
    -a, --all
            Commit all files in the repository. This is the default.
            
            The commit will fail if there are no changes, unless `--allow-empty`
            is set.

    -s, --staged
            Commit only files that have been explicitly staged with `git add`.
            
            This is like the default behaviour of `git commit`. The commit will
            fail if there are no staged changes unless `--allow-empty` is set.

        --tree <TREE>
            Include the specified tree object in the commit, without looking at
            or modifying the index or working tree

    -e, --empty
            Don't include any file changes in the commit.
            
            This commit will have the same tree hash as its parent.

        --allow-empty
            Create the commit even if it contains no changes
            
            [env: SAVE_ALLOW_EMPTY=]

COMMIT OPTIONS:
    -m, --message <MESSAGE>
            The commit message.
            
            [default: a short string based on the commit's tree hash and
            ancestry graph]
            
            [env: SAVE_COMMIT_MESSAGE=]

    -M, --message-prefix <MESSAGE_PREFIX>
            A prefix to put on its own line before the commit message. This is
            typically only useful if you're squashing/amending commits with
            existing messages you want to add to
            
            [env: SAVE_COMMIT_MESSAGE_PREFIX=]

    -x, --prefix <PREFIX_HEX>
            The required commit ID hash or prefix, in hex. This will be
            brute-forced.
            
            This supports some non-hex values with special meanings:
            
            - `_` is skipped, for a character whose value we don't care about.
            - 'C' is replaced by the next nibble of the
            minimum-timestamped-variant commit ID.
            - 'R' is replaced with the last digits of the revision index.
            - 'G' is replaced with the last digits of the generation index.
            - 'N' is replaced with the last digits of the commit index.
            
            May be explicitly set to an empty string to skip brute-forcing the
            hash.
            
            [default: "CCCC", representing the first four hex digits of the
            commit's tree hash]
            
            [env: SAVE_COMMIT_PREFIX=]

        --head <HEAD>
            What branch head are we updating? Defaults to `"HEAD"` (which also
            updates the current branch if one is checked out). Setting it to any
            value name will create or force-update that branch without modifying
            HEAD or the working directory
            
            [env: SAVE_HEAD=]

    -n, --no-head
            Prepare the commit, but don't actually update any references in Git.
            
            The commit will be written to the Git database, so it is still
            possible for the user to manually add a reference to it.
            
            [env: SAVE_NO_HEAD=]
            [aliases: dry-run]

SIGNATURE OPTIONS:
    -t, --timestamp <TIMESTAMP>
            Override the system clock timestamp value
            
            [env: SAVE_TIMESTAMP=]

    -0, --timeless
            Use the next available timestamp after the parent commit's
            timestamps,  regardless of the actual current clock time. Assuming
            there is a parent  commit, this is equivalent to `--timestamp=0`. If
            we're creating an  initial commit (with no parents), this uses the
            next available timestamp  after the current time (or value provided
            to `--timestamp`) rounded down  to the closest multiple of
            `0x1000000` (a period of ~6 months).
            
            This can be used to help produce deterministic timestamps and commit
            IDs for reproducible builds.
            
            [env: SAVE_TIMELESS=]

        --author <AUTHOR>
            The name and email to use for the commit's author.
            
            [default: name from git, or else from parent commit, or else "user
            <user@localhost>"]
            
            [env: SAVE_AUTHOR=]

        --committer <COMMITTER>
            The name and email to use for the commit's committer.
            
            [default: copied from the commit author]
            
            [env: SAVE_COMMITTER=]

HISTORY OPTIONS:
    -p, --add-parent <ADDED_PARENT_REF>
            Adds another parent to the new commit. May be repeated to add
            multiple parents, though duplicated parents will are ignored
            
            [env: SAVE_ADD_PARENT=]

        --remove-parent <REMOVED_PARENT_REF>
            Removes a parent from the new commit. May be repeated to remove
            multiple parents. If the parent is not present, this will fail with
            an error
            
            [env: SAVE_REMOVE_PARENTS=]

    -u, --squash
            Squashes these changes into the first parent. May be repeated
            multiple times to squash multiple generations. Authors of squashed
            commits will be added using the Co-Authored-By header
            
            [env: SAVE_SQUASH_COUNT=]
            [aliases: amend]

        --squash-to <SQUASH_TO_REF>
            Squashes all changes from this commit up to the specified ancestor
            commit(s). Authors of squashed commits will be added using the
            Co-Authored-By header.
            
            This will fail if the specified commit isn't actually an ancestor.
            
            [env: SAVE_SQUASH_TO=]

        --squash-after <SQUASH_AFTER_REF>
            Squashes every ancestor commit that isn't part included in the
            target head(s).
            
            For example, this can be used to squash all changes in a branch by
            excluding the upstream branch.
            
            [env: SAVE_SQUASH_AFTER=]

        --squash-all <SQUASH_ALL>
            Squashes the entire repository into a single commit. You probably
            don't want to use this. If you really do, you must set this flag to
            the value `CONFIRM_SQUASH_ALL`
            
            [env: SAVE_SQUASH_ALL=]

        --retcon-to-ref <RETCON_TO_REF>
            Rewrites the timestamps and authorship information of all commits up
            to the given ancestors based on the current settings.
            
            Commit messages will only be replaced if they match our generated
            message pattern, or are empty.
            
            [env: SAVE_RETCON_TO=]

        --retcon-after <RETCON_AFTER_REF>
            Retcons every ancestor commit that isn't part included in the target
            head(s).
            
            For example, this can be used to retcon all changes in a branch by
            excluding the upstream branch.
            
            [env: SAVE_RETCON_AFTER=]

        --retcon-all
            Retcons the entire history. You probably don't want to use this, but
            if you do use it consistently it should only affect the most recent
            commit
            
            [env: SAVE_RETCON_ALL=]

INSTALLATION:
    save can be installed from a source release using the Cargo package manager:

        cargo install save --version 0.20220708.0

    Cargo can be installed along with Rust/rustup using its official installer:

        curl -sSf https://sh.rustup.rs | sh

LINKS:
    https://docs.rs/save/0.20220708.0
    https://crates.io/crates/save/0.20220708.0
