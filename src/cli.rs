//! The CLI.

use {
    crate::git2::*,
    ::{
        clap::{AppSettings, Parser},
        eyre::{bail, Result},
        git2::{
            Commit, ErrorCode, Repository, RepositoryInitOptions, RepositoryState, Signature, Time,
        },
        once_cell::sync::Lazy,
        std::{env, fmt::Write, fs, process::Command},
        tracing::{debug, info, instrument, trace, warn},
    },
};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const V_VERSION: &'static str = concat!("v", env!("CARGO_PKG_VERSION"));

/// Would you like to SAVE the change?
///
/// Commit everything in the current Git repository, no questions asked.
#[derive(Parser, Debug, Clone, Default)]
#[clap(
    after_help = {
        static AFTER_HELP: Lazy<String> = Lazy::new(|| format!("LINKS:
    https://docs.rs/save/{VERSION}
    https://crates.io/crates/save/{VERSION}"));
        AFTER_HELP.as_ref()
    },
    dont_collapse_args_in_usage = true,
    infer_long_args = true,
    setting = AppSettings::DeriveDisplayOrder,
    version = V_VERSION
)]
#[non_exhaustive]
pub struct Args {
    // GENERAL OPTIONS:
    /// Decrease log verbosity. May be repeated to decrease verbosity further.
    #[clap(long, short = 'q', parse(from_occurrences))]
    pub quiet: i32,

    /// Increase log verbosity. May be repeated to increase verbosity further.
    #[clap(long, short = 'v', parse(from_occurrences))]
    pub verbose: i32,

    // CONTENT OPTIONS:
    // //
    // //
    // //
    /// Commit all files in the repository. This is the default.
    ///
    /// The commit will fail if there are no changes, unless `--allow-empty` is
    /// set.
    #[clap(long, help_heading="CONTENT OPTIONS", short = 'a', conflicts_with_all = &["staged", "tree", "empty"])]
    pub all: bool,

    /// Commit only files that have been explicitly staged with `git add`.
    ///
    /// This is like the default behaviour of `git commit`.
    /// The commit will fail if there are no staged changes unless
    /// `--allow-empty` is set.
    #[clap(long, help_heading="CONTENT OPTIONS", short = 's', conflicts_with_all = &["all", "tree", "empty"])]
    pub staged: bool,

    /// Include the specified tree object in the commit, without looking at or
    /// modifying the index or working tree.
    #[clap(long, help_heading="CONTENT OPTIONS", conflicts_with_all = &["all", "staged", "empty"])]
    pub tree: Option<String>,

    /// Don't include any file changes in the commit.
    ///
    /// This commit will have the same tree hash as its parent.
    #[clap(long, help_heading="CONTENT OPTIONS", short = 'e', conflicts_with_all = &["all", "staged", "tree"])]
    pub empty: bool,

    /// Create the commit even if it contains no changes.
    #[clap(long, help_heading = "CONTENT OPTIONS", env = "SAVE_ALLOW_EMPTY")]
    pub allow_empty: bool,

    // COMMIT OPTIONS:
    /// The commit message.
    ///
    /// [default: a short string based on the commit's tree hash and ancestry
    /// graph]
    #[clap(
        long,
        help_heading = "COMMIT OPTIONS",
        short = 'm',
        env = "SAVE_COMMIT_MESSAGE",
        conflicts_with = "message-prefix"
    )]
    pub message: Option<String>,

    /// A prefix to put on its own line before the commit message. This is
    /// typically only useful if you're squashing/amending commits with
    /// existing messages you want to add to.
    #[clap(
        long,
        help_heading = "COMMIT OPTIONS",
        short = 'M',
        env = "SAVE_COMMIT_MESSAGE_PREFIX"
    )]
    pub message_prefix: Option<String>,

    /// The required commit ID hash or prefix, in hex. This will be
    /// brute-forced.
    ///
    /// This supports some non-hex values with special meanings:
    ///
    /// - `_` underscore skips a character whose value we don't care about.
    /// - 'T' is replaced with the next nibble of the tree hash.
    /// - 'R' is replaced with the last digits of the revision index.
    /// - 'G' is replaced with the last digits of the generation index.
    /// - 'N' is replaced with the last digits of the commit index.
    ///
    /// [default: "TTTT", representing the first four hex digits of the commit's
    /// tree hash]
    #[clap(
        long = "prefix",
        help_heading = "COMMIT OPTIONS",
        short = 'x',
        env = "SAVE_COMMIT_PREFIX"
    )]
    pub prefix_hex: Option<String>,

    // SIGNATURE OPTIONS:
    /// Override the system clock timestamp value.
    #[clap(
        long,
        help_heading = "SIGNATURE OPTIONS",
        short = 't',
        env = "SAVE_TIMESTAMP"
    )]
    pub timestamp: Option<i64>,

    /// Use the next available timestamp after the parent commit's timestamps,
    /// regardless of the actual current clock time. Assuming there is a parent
    /// commit, this is equivalent to `--timestamp=0`. If we're creating an
    /// initial commit (with no parents), this uses the next available timestamp
    /// after the current time (or value provided to `--timestamp`) rounded down
    /// to the closest multiple of `0x1000000` (a period of ~6 months).
    ///
    /// This can be used to help produce deterministic timestamps and commit
    /// IDs for reproducible builds.
    #[clap(
        long,
        help_heading = "SIGNATURE OPTIONS",
        short = '0',
        env = "SAVE_TIMELESS"
    )]
    pub timeless: bool,

    /// The name and email to use for the commit's author.
    ///
    /// [default: name from git, or else from parent commit, or else "user"]
    #[clap(long, help_heading = "SIGNATURE OPTIONS", env = "SAVE_AUTHOR")]
    pub author: Option<String>,

    /// The name and email to use for the commit's committer.
    ///
    /// [default: copied from the commit author]
    #[clap(long, help_heading = "SIGNATURE OPTIONS", env = "SAVE_COMMITTER")]
    pub committer: Option<String>,

    // // // // HISTORY OPTIONS // // // //
    /// What branch head are we updating? Defaults to `"HEAD"` (which also
    /// updates the current branch if one is checked out). Setting it to any
    /// value name will create or force-update that branch without modifying
    /// HEAD or the working directory.
    #[clap(
        long,
        help_heading = "COMMIT OPTIONS",
        env = "SAVE_HEAD",
        conflicts_with = "no-head"
    )]
    pub head: Option<i64>,

    /// Prepare the commit, but don't actually update any references in Git.
    ///
    /// The commit will be written to the Git database, so it is still possible
    /// for the user to manually add a reference to it.
    #[clap(
        long,
        help_heading = "COMMIT OPTIONS",
        short = 'n',
        visible_alias = "dry-run",
        conflicts_with = "head"
    )]
    pub no_head: bool,

    /// Adds another parent to the new commit. May be repeated to add multiple
    /// parents, though duplicated parents will are ignored.
    #[clap(long = "add-parent", help_heading = "HISTORY OPTIONS", short = 'p')]
    pub added_parent_ref: Vec<String>,

    /// Removes a parent from the new commit. May be repeated to remove multiple
    /// parents. If the parent is not present, this will fail with an error.
    #[clap(long = "remove-parent", help_heading = "HISTORY OPTIONS")]
    pub removed_parent_ref: Vec<String>,

    /// Squashes these changes into the first parent. May be repeated multiple
    /// times to squash multiple generations. Authors of squashed commits will
    /// be added using the Co-Authored-By header.
    #[clap(
        long,
        help_heading = "HISTORY OPTIONS",
        short = 'u',
        parse(from_occurrences),
        visible_alias = "amend",
        conflicts_with = "squash-tail-ref"
    )]
    pub squash: u32,

    /// Squashes all changes from this commit up to the specified ancestor
    /// commit(s). Authors of squashed commits will be added using the
    /// Co-Authored-By header.
    ///
    /// This will fail if the specified commit isn't actually an ancestor.
    #[clap(
        long = "squash-tail",
        help_heading = "HISTORY OPTIONS",
        conflicts_with = "squash"
    )]
    pub squash_tail_ref: Vec<String>,

    /// Squashes every ancestor commit that isn't part included in the target
    /// head(s).
    ///
    /// For example, this can be used to squash all changes in a branch by
    /// excluding the upstream branch.
    #[clap(
        long = "squash-after-head", help_heading = "HISTORY OPTIONS", conflicts_with_all = &["squash-tail-ref", "retcon-all"]
    )]
    pub squash_after_head_ref: Vec<String>,

    /// Rewrites the timestamps and authorship information of all commits up to
    /// the given ancestors based on the current settings.
    ///
    /// Commit messages will only be replaced if they match our generated
    /// message pattern, or are empty.
    #[clap(
        long = "retcon-tail", help_heading = "HISTORY OPTIONS", conflicts_with_all = &["retcon-after-head-ref", "retcon-all"]
    )]
    pub retcon_tail_ref: Vec<String>,

    /// Retcons every ancestor commit that isn't part included in the target
    /// head(s).
    ///
    /// For example, this can be used to retcon all changes in a branch by
    /// excluding the upstream branch.
    #[clap(long = "retcon-after-head", help_heading = "HISTORY OPTIONS", conflicts_with_all = &["retcon-tail-ref", "retcon-all"]
)]
    pub retcon_after_head_ref: Vec<String>,

    /// Retcons the entire history. You probably don't want to use this,
    /// but if you do use it consistently it should only affect the most
    /// recent commit.
    #[clap(long, help_heading = "HISTORY OPTIONS", conflicts_with_all = &["retcon-tail-ref", "retcon-after-head-ref"])]
    pub retcon_all: bool,
}

impl Args {
    pub fn with<F: FnOnce(&mut Self) -> T, T>(f: F) -> Self {
        let mut args = Self::default();
        f(&mut args);
        args
    }
}

/// CLI entry point.
#[instrument(level = "debug", skip(args))]
pub fn main(args: Args) -> Result<()> {
    let repo = open_or_init_repo(&args)?;

    let args = Args::with(|args| {
        args.retcon_all = true;
    });

    // TODO: move most of the following to RepositoryExt::Save

    let head = match repo.head() {
        Ok(head) => Some(head.peel_to_commit().unwrap()),
        Err(err) if err.code() == ErrorCode::UnbornBranch => None,
        Err(err) => {
            bail!("Unexpected error from Git: {:#?}", err);
        },
    };

    let (user_name, user_email) = get_git_user(&args, &repo, &head)?;

    let graph_stats = head
        .as_ref()
        .map(|commit| commit.graph_stats())
        .unwrap_or_default();

    let mut index = repo.working_index()?;

    let tree = index.write_tree()?;

    if let Some(ref head) = head {
        if tree == head.tree_id() {
            if args.message.is_some() {
                info!("Committing with only a message.");
            } else if args.empty || args.allow_empty {
                info!("Committing with no changes.");
            } else {
                warn!("Nothing to commit. Use --empty or --allow-empty if this is intentional.");
                return Ok(());
            }
        }
    }

    if !args.no_head {
        index.write()?;
    } else {
        info!("Skipping index write because this is a dry run.");
    }

    let tree4 = tree.to_string()[..4].to_string().to_ascii_uppercase();

    let target = crate::hex::decode_hex_nibbles(args.prefix_hex.unwrap_or_else(|| tree4.clone()));

    let tree = repo.find_tree(tree)?;

    let mut message = String::new();
    write!(message, "r{}", graph_stats.revision_index)?;

    if graph_stats.generation_index != graph_stats.revision_index {
        write!(message, " / g{}", graph_stats.generation_index)?;
    }

    if graph_stats.commit_index != graph_stats.generation_index {
        write!(message, " / n{}", graph_stats.commit_index)?;
    }

    if !tree.is_empty() {
        write!(message, " / x{tree4}")?;
    }

    // TODO: look at merge heads too, and set our minimum timestamp to one greater
    // than the maximum of all heads
    let previous_seconds = head.as_ref().map(|c| c.time().seconds()).unwrap_or(0);
    let time = Signature::now(&user_name, &user_email)?.when();
    let seconds = time.seconds();

    let parents = &head.iter().collect::<Vec<_>>();

    let base_commit = repo.commit(
        None,
        &Signature::new(&user_name, &user_email, &Time::new(seconds, 0)).unwrap(),
        &Signature::new(&user_name, &user_email, &Time::new(seconds, 0)).unwrap(),
        &message,
        &tree,
        parents,
    )?;
    let base_commit = repo.find_commit(base_commit)?;

    let min_timestamp = previous_seconds;
    let target_timestamp = seconds;

    let commit = base_commit.brute_force_timestamps(
        &repo,
        &target.bytes,
        Some(&target.mask),
        min_timestamp,
        target_timestamp,
    );

    debug!("Prepared commit {}", commit.id());

    if !args.no_head {
        let mut head_ref = repo.head()?;
        info!("Updating HEAD: {}", head_ref.shorthand().unwrap());
        if head_ref.is_branch() {
            head_ref.set_target(commit.id(), "committed via save")?;
        } else {
            repo.set_head(&commit.id().to_string())?;
        }
    } else {
        info!("Not updating HEAD because this is a dry run.");
    }

    eprintln!();

    Command::new("git")
        .args(&[
            "--no-pager",
            "log",
            "--name-status",
            "--format=fuller",
            "--date=human-local",
            "--graph",
            "--decorate",
            "-n",
            "2",
            &commit.id().to_string(),
        ])
        .status()?;

    eprintln!();

    Command::new("git")
        .args(&["--no-pager", "reflog", "-n", "4"])
        .status()?;

    eprintln!();

    Ok(())
}

/// Determine the Git user name and email to use.
/// XXX: This should be removed or merged into git2.rs.
#[instrument(level = "debug", skip(repo))]
fn get_git_user(args: &Args, repo: &Repository, head: &Option<Commit>) -> Result<(String, String)> {
    // TODO: move this to git2.rs, right?

    let config = repo.config()?;

    let user_name: String = {
        if let Some(ref args_name) = args.author {
            trace!(
                "Using author name from command line argument: {:?}",
                &args_name
            );
            args_name.clone()
        } else if let Ok(config_name) = config.get_string("user.name") {
            debug!(
                "Using author name from Git configuration: {:?}",
                &config_name
            );
            config_name
        } else if let Some(previous_name) = head
            .as_ref()
            .and_then(|x| x.author().name().map(std::string::ToString::to_string))
        {
            info!(
                "Using author name from previous commit: {:?}",
                &previous_name
            );
            previous_name
        } else {
            let placeholder_name = "dev";
            warn!(
                "No author name found, falling back to placeholder: {:?}",
                &placeholder_name
            );
            placeholder_name.to_string()
        }
    };

    let user_email: String = if let Some(ref args_email) = args.author {
        trace!(
            "Using author email from command line argument: {:?}",
            &args_email
        );
        args_email.clone()
    } else if let Ok(config_email) = config.get_string("user.email") {
        debug!(
            "Using author email from Git configuration: {:?}",
            &config_email
        );
        config_email
    } else if let Some(previous_email) = head
        .as_ref()
        .and_then(|x| x.author().email().map(std::string::ToString::to_string))
    {
        info!(
            "Using author email from previous commit: {:?}",
            &previous_email
        );
        previous_email
    } else {
        let placeholder_email = "dev@localhost";
        warn!(
            "No author email found, falling back to placeholder: {:?}",
            &placeholder_email
        );
        placeholder_email.to_string()
    };

    Ok((user_name, user_email))
}

/// Opens or initializes a new [git2::Repository] in CWD or GIT_DIR, if args
/// allow it.
/// XXX: This should be removed or merged into git2.rs.
#[instrument(level = "debug")]
fn open_or_init_repo(args: &Args) -> Result<Repository> {
    let repo = match Repository::open_from_env() {
        Ok(repo) => {
            if repo.is_bare() {
                bail!(
                    "Found Git repository, but it was bare (no working directory): {:?}",
                    repo.path()
                );
            }

            debug!("Found Git repository: {:?}", repo.workdir().unwrap());
            repo
        },
        Err(_err) => {
            let path = std::env::current_dir()?;
            let empty = fs::read_dir(&path)?.next().is_none();
            info!("No Git repository found.");

            let dangerous = (path == home::home_dir().unwrap()) || (path.to_str() == Some("/"));

            if dangerous {
                bail!(
                    "Current directory seems important, refusing to run `git init` automatically."
                );
            } else if empty && !args.empty {
                bail!("Current directory is empty, skipping auto-init (--empty to override).");
            } else {
                info!("Initializing a new Git repository in: {:?}", path);
                if args.no_head {
                    bail!("Can't initialize a new repository on a dry run.");
                }
                Repository::init_opts(
                    path,
                    RepositoryInitOptions::new()
                        .initial_head("trunk")
                        .no_reinit(true),
                )?
            }
        },
    };

    if repo.state() != RepositoryState::Clean {
        bail!(
            "Repository is in the middle of another operation: {:?}",
            repo.state()
        );
    }

    Ok(repo)
}

/// Initialize the typical global environment and parses the typical [Args] for
/// save's [main] CLI entry point.
///
/// # Panics
///
/// This will panic if called multiple times, or if other code attempts
/// conflicting global initialization of systems such as logging.
#[must_use]
pub fn init() -> Args {
    ::color_eyre::install().unwrap();

    let args = Args::parse();

    let default_verbosity_self = 3;
    let default_verbosity_other = 1;

    let log_env = env::var("RUST_LOG").unwrap_or_default();

    let rust_log = if args.verbose == 0 && args.quiet == 0 && !log_env.is_empty() {
        log_env
    } else {
        let verbosity_self = match default_verbosity_self + args.verbose - args.quiet {
            i32::MIN..=0 => "off",
            1 => "error",
            2 => "warn",
            3 => "info",
            4 => "debug",
            5..=i32::MAX => "trace",
        };
        let verbosity_other = match default_verbosity_other + args.verbose - args.quiet {
            i32::MIN..=0 => "off",
            1 => "error",
            2 => "warn",
            3 => "info",
            4 => "debug",
            5..=i32::MAX => "trace",
        };
        format!("{verbosity_other},save={verbosity_self}")
    };

    ::tracing_subscriber::util::SubscriberInitExt::init(
        tracing_subscriber::Layer::with_subscriber(
            ::tracing_error::ErrorLayer::default(),
            ::tracing_subscriber::fmt()
                .with_env_filter(::tracing_subscriber::EnvFilter::new(rust_log))
                .with_target(false)
                .with_span_events(
                    tracing_subscriber::fmt::format::FmtSpan::ENTER
                        | tracing_subscriber::fmt::format::FmtSpan::CLOSE,
                )
                .compact()
                .finish(),
        ),
    );

    trace!("Initialized from: {:#?}", args);

    args
}
