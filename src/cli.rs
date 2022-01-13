//! CLI logic

use {
    crate::git::*,
    clap::Parser,
    digest::Digest,
    eyre::{bail, Result, WrapErr},
    git2::{
        Commit, ErrorCode, Index, Repository, RepositoryInitOptions, RepositoryState, Signature,
        Time,
    },
    rayon::prelude::*,
    std::{env, fs, path::PathBuf, process::Command},
    tracing::{debug, info, instrument, trace, warn},
};

/// Would you like to SAVE the change?
///
/// Commit everything in the current Git repository, no questions asked.
#[derive(Parser, Debug, Clone)]
#[clap(version, max_term_width = max_term_width())]
pub struct Args {
    /// Commit message to use.
    ///
    /// [default: generated from generation number, tree hash, and parents]
    #[clap(long, short = 'm')]
    pub message: Option<String>,

    /// Prepare the commit, but don't actually update any references in Git.
    #[clap(long, short = 'n')]
    pub dry_run: bool,

    /// Proceed in spite of any warnings.
    #[clap(long, short = 'y')]
    pub yes: bool,

    /// Squash/amend previous commit(s), instead of adding a new one.
    ///
    /// By default, `--squash` will behave like `git commit --amend`, only
    /// replacing the most recent commit. However, specifying a larger number
    /// such as `--squash=2` will squash that many recent first-parents (and
    /// any current changes) into a single commit. If any of those commits are
    /// merges, any non-squashed parents will be added as parents of the
    /// squashed commit. Any additional authors will be included in
    /// Co-Authored-By footers. Commit messages will be discarded.
    #[clap(
        long = "squash",
        short = 's',
        default_value = "0",
        default_missing_value = "1"
    )]
    pub squash_commits: u32,

    /// The target commit hash or prefix, in hex.
    ///
    /// [default: the commit's tree hash]
    #[clap(long = "hash", short = 'x')]
    pub hash_hex: Option<String>,

    /// The name to use for the commit's author and committer.
    ///
    /// [default: name from git, or else from parent commit, or else "save"]
    #[clap(long = "name")]
    pub name: Option<String>,

    /// The email to use for the commit's author and committer.
    ///
    /// [default: email from git, or else from parent commit, or else "save"]
    #[clap(long)]
    pub email: Option<String>,

    /// The time is NOW.
    ///
    /// [default: the time is ACTUALLY now]
    #[clap(long = "now", short = 'w')]
    pub now_seconds: Option<i64>,

    /// Seconds of timestamp allocated for each commit to search.
    ///
    /// The number of possibilities searched is the half the square of this
    /// value.
    #[clap(long = "step", short = 't', default_value_t = 128)]
    pub step_seconds: u32,

    /// Decrease log verbosity. May be used multiple times.
    #[clap(long, short = 'q', parse(from_occurrences))]
    pub quiet: i32,

    /// Increase log verbosity. May be used multiple times.
    #[clap(long, short = 'v', parse(from_occurrences))]
    pub verbose: i32,
}

/// Used to override the max_term_width of our derived [Args]
/// using the **build time** environment variable MAX_TERM_WIDTH.
///
/// This is hacky and bad for the build cache, only meant for internal use in
/// generating the `--help` text for `README.md`, which needs to be
/// tightly wrapped to fit in available space on crates.io.
fn max_term_width() -> usize {
    option_env!("MAX_TERM_WIDTH")
        .unwrap_or("100")
        .parse()
        .unwrap()
}

/// CLI entry point.
///
/// # Panics
///
/// For some fatal errors.
///
/// # Errors
///
/// For other fatal errors.
#[instrument(level = "debug", skip(args))]
pub fn main(args: Args) -> Result<()> {
    if args.squash_commits > 0 {
        todo!("--squash has not been implemented");
    }

    let mut target_hash = args
        .hash_hex
        .as_ref()
        .map(|s| hex::decode(s).wrap_err("target hash must be hex").unwrap())
        .unwrap_or_default();

    let repo = open_or_init_repo(&args)?;

    let head = match repo.head() {
        Ok(head) => Some(head.peel_to_commit().unwrap()),
        Err(err) if err.code() == ErrorCode::UnbornBranch => None,
        Err(err) => {
            bail!("Unexpected error from Git: {:#?}", err);
        },
    };

    let (user_name, user_email) = get_git_user(&args, &repo, &head)?;

    let generation_number = head
        .as_ref()
        .map(|commit| commit.generation_number() + 1)
        .unwrap_or(0);

    let mut index = new_index(&repo)?;

    let tree = index.write_tree()?;

    if let Some(ref head) = head {
        if tree == head.tree_id() {
            if args.message.is_some() {
                info!("Committing with only a message.");
            } else if args.yes {
                info!("Committing with no changes.");
            } else {
                info!("Nothing to commit (-y/--yes to override).");
                return Ok(());
            }
        }
    }

    if !args.dry_run {
        index.write()?;
    } else {
        info!("Skipping index write because this is a dry run.");
    }

    let tree4 = &tree.to_string()[..4];
    let tree = repo.find_tree(tree)?;

    let revision_index = generation_number + 1;
    let message = args.message.unwrap_or_else(|| {
        let mut message = format!("r{}", revision_index);
        if let Some(ref head) = head {
            message += &format!("/{}/{}", tree4, &head.id().to_string()[..4]);
        } else if tree.iter().next().is_some() {
            message += &format!("/{}", &tree4);
        }
        message
    });

    let previous_seconds = head.as_ref().map(|c| c.time().seconds()).unwrap_or(0);
    let time = Signature::now(&user_name, &user_email)?.when();
    let mut seconds = args.now_seconds.unwrap_or_else(|| time.seconds());
    let offset = 0;

    let seconds_since_head = seconds - previous_seconds;

    let step_seconds = i64::from(args.step_seconds);
    let snap_seconds = step_seconds * 2;
    let slack_seconds = step_seconds * 4;

    if seconds_since_head < slack_seconds {
        seconds = previous_seconds + step_seconds;
    } else {
        seconds = seconds - seconds % snap_seconds;
    }

    let parents = &head.iter().collect::<Vec<_>>();

    let min_timestamp = seconds;
    let max_timestamp = seconds + step_seconds - 1;

    target_hash.append(&mut tree.id().as_bytes().to_vec());

    let base_commit = repo.commit(
        None,
        &Signature::new(&user_name, &user_email, &Time::new(min_timestamp, offset)).unwrap(),
        &Signature::new(&user_name, &user_email, &Time::new(min_timestamp, offset)).unwrap(),
        &message,
        &tree,
        parents,
    )?;
    let base_commit = repo.find_commit(base_commit)?;

    let commit = base_commit.brute_force_timestamps(&target_hash, min_timestamp, max_timestamp);

    let commit = commit.commit();

    if !args.dry_run {
        let mut head_ref = repo.head()?;
        if head_ref.is_branch() {
            head_ref.set_target(commit.id(), "committed via save")?;
        } else {
            repo.set_head(&commit.id().to_string())?;
        }
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
        ])
        .status()?;

    eprintln!();

    Ok(())
}

// #[instrument(level = "x", skip(repo))]
fn squashed_commit<'a>(_repo: &'a mut Repository, _head: &'a Commit, _depth: usize) -> Commit<'a> {
    todo!();
}

/// Determine the Git user name and email to use.
#[instrument(level = "debug", skip(repo))]
fn get_git_user(args: &Args, repo: &Repository, head: &Option<Commit>) -> Result<(String, String)> {
    let config = repo.config()?;

    let user_name: String = {
        if let Some(ref args_name) = args.name {
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
            let placeholder_name = "save";
            warn!(
                "No author name found, falling back to placeholder: {:?}",
                &placeholder_name
            );
            placeholder_name.to_string()
        }
    };

    let user_email: String = if let Some(ref args_email) = args.email {
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
        let placeholder_email = "save";
        warn!(
            "No author email found, falling back to placeholder: {:?}",
            &placeholder_email
        );
        placeholder_email.to_string()
    };

    Ok((user_name, user_email))
}

/// Generates an updated [git2::Index] with every file in the directory.
#[instrument(level = "debug", skip(repo))]
fn new_index(repo: &Repository) -> Result<Index> {
    let mut index = repo.index()?;
    index
        .add_all(
            ["*"],
            Default::default(),
            Some(&mut |path, _| {
                if path.to_string_lossy().ends_with('/') {
                    let mut git_path = PathBuf::from(path);
                    git_path.push(".git");
                    if git_path.is_dir() {
                        warn!(
                            "Encountered a Git submodule; skipping it: {}",
                            git_path.to_string_lossy()
                        );
                        return 1;
                    }
                }
                trace!("Adding: {}", path.to_string_lossy());
                0
            }),
        )
        .wrap_err("Failed to add something to the Git index.")?;
    Ok(index)
}

/// Opens or initializes a new [git2::Repository] in CWD or GIT_DIR, if args
/// allow it.
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

            if dangerous && !args.yes {
                bail!(
                    "Current directory seems important, skipping auto-init (-y/--yes to override)."
                );
            } else if empty && !args.yes {
                bail!("Current directory is empty, skipping auto-init (-y/--yes to override).");
            } else {
                info!("Initializing a new Git repository in: {:?}", path);
                if args.dry_run {
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

/// Brute forces timestamps for a raw Git commit.
///
/// Given a raw Git commit as a string, finds the timestamps in the given range
/// that will produce the closest commit ID to target_hash. We search possible
/// commits where
/// `min_timestamp <= author_timestamp <= committer_timestamp <= max_timestamp`.
#[instrument(level = "debug")]
pub fn brute_force_timestamps(
    base_commit: &str,
    target_hash: &[u8],
    min_timestamp: i64,
    max_timestamp: i64,
) -> (i64, i64) {
    let base_commit_lines = base_commit.split('\n').collect::<Vec<&str>>();
    let author_line_index = base_commit_lines
        .iter()
        .position(|line| line.starts_with("author "))
        .expect("author line missing in commit");
    let author_line_pieces = &base_commit_lines[author_line_index]
        .split(' ')
        .collect::<Vec<_>>();
    let committer_line_index = base_commit_lines
        .iter()
        .position(|line| line.starts_with("committer "))
        .expect("committer line missing in commit");
    let committer_line_pieces = &base_commit_lines[committer_line_index]
        .split(' ')
        .collect::<Vec<_>>();

    let commit_create_buffer = |author_timestamp: i64, committer_timestamp: i64| {
        let mut commit_lines = base_commit_lines.clone();

        let mut author_line_pieces = author_line_pieces.clone();
        let i = author_line_pieces.len() - 2;
        let author_timestamp = author_timestamp.to_string();
        author_line_pieces[i] = &author_timestamp;
        let author_line = author_line_pieces.join(" ");
        commit_lines[author_line_index] = &author_line;

        let mut committer_line_pieces = committer_line_pieces.clone();
        let i = committer_line_pieces.len() - 2;
        let committer_timestamp = committer_timestamp.to_string();
        committer_line_pieces[i] = &committer_timestamp;
        let committer_line = committer_line_pieces.join(" ");
        commit_lines[committer_line_index] = &committer_line;

        commit_lines.join("\n")
    };

    let (_score, author_timestamp, commit_timestamp, hash, _candidate) = ((min_timestamp
        ..=max_timestamp)
        .into_par_iter()
        .map(|author_timestamp| {
            (author_timestamp..=max_timestamp)
                .into_par_iter()
                .map(|commit_timestamp| {
                    let candidate = commit_create_buffer(author_timestamp, commit_timestamp);
                    let hash = sha1::Sha1::new()
                        .chain_update(format!("commit {}", candidate.len()))
                        .chain_update([0x00])
                        .chain_update(&candidate)
                        .finalize()
                        .to_vec();

                    let score = hash
                        .iter()
                        .zip(target_hash.iter())
                        .map(|(a, b)| (a ^ b))
                        .collect::<Vec<u8>>();

                    (score, author_timestamp, commit_timestamp, hash, candidate)
                })
                .min()
                .unwrap()
        }))
    .min()
    .unwrap();

    debug!(
        "Brute-forced a commit with id: {}",
        hash.iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join("")
    );

    (author_timestamp, commit_timestamp)
}

/// Initialize the typical global environment and parses the typical [Args] for
/// save's [main] CLI entry point.
///
/// # Panics
///
/// This will panic if called multiple times, or if other code attempts
/// conflicting global initialization of systems such as logging.
#[must_use] pub fn init() -> Args {
    color_eyre::install().unwrap();

    let args = Args::parse();

    let default_verbosity = 3;

    let log_env = env::var("RUST_LOG").unwrap_or_default();

    let log_level = if args.verbose == 0 && args.quiet == 0 && !log_env.is_empty() {
        log_env
    } else {
        match default_verbosity + args.verbose - args.quiet {
            i32::MIN..=0 => "off".into(),
            1 => "error".into(),
            2 => "warn".into(),
            3 => "info".into(),
            4 => "debug".into(),
            5..=i32::MAX => "trace".into(),
        }
    };

    tracing_subscriber::util::SubscriberInitExt::init(tracing_subscriber::Layer::with_subscriber(
        tracing_error::ErrorLayer::default(),
        tracing_subscriber::fmt()
            .with_env_filter(::tracing_subscriber::EnvFilter::new(log_level))
            .with_target(false)
            .with_span_events(
                tracing_subscriber::fmt::format::FmtSpan::ENTER
                    | tracing_subscriber::fmt::format::FmtSpan::CLOSE,
            )
            .compact()
            .finish(),
    ));

    trace!("Initialized from: {:#?}", args);

    args
}
