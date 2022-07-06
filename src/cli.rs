//! The CLI.

use {
    crate::git2::*,
    clap::{AppSettings, Parser},
    eyre::{bail, Result},
    git2::{
        Commit, ErrorCode, Repository, RepositoryInitOptions, RepositoryState, Signature, Time,
    },
    once_cell::sync::Lazy,
    std::{env, fmt::Write, fs, process::Command},
    tracing::{debug, info, instrument, trace, warn},
};

/// Would you like to SAVE the change?
///
/// Commit everything in the current Git repository, no questions asked.
#[derive(Parser, Debug, Clone)]
#[clap(
    after_help = {
        static AFTER_HELP: Lazy<String> = Lazy::new(|| { format!(
            "{}\n    https://docs.rs/{name}/{semver}\n    https://crates.io/crates/{name}\n    {repository}",
            "LINKS:",
            name = env!("CARGO_PKG_NAME"),
            semver = {
                if env!("CARGO_PKG_VERSION_PRE", "").len() > 0 {
                    format!("%3C%3D{}", env!("CARGO_PKG_VERSION"))
                } else {
                    env!("CARGO_PKG_VERSION").to_string()
                }
            },
            repository = env!("CARGO_PKG_REPOSITORY"))
        });
        AFTER_HELP.as_ref()
    },
    dont_collapse_args_in_usage = true,
    infer_long_args = true,
    setting = AppSettings::DeriveDisplayOrder,
    version
)]
pub struct Args {
    /// Use this commit message, instead of the default.
    ///
    /// [default: generated from generation number, tree hash, and parents]
    #[clap(long, short = 'm', env = "SAVE_COMMIT_MESSAGE")]
    pub message: Option<String>,

    /// Adds another parent to this commit. May be used multiple times.
    #[clap(long = "add-parent")]
    pub add_parent: Vec<String>,

    /// Commit all files in the repository. This is the default.
    #[clap(long = "all", short = 'a', conflicts_with = "empty")]
    pub all: bool,

    /// Don't include any file changes in the commit.
    ///
    /// This commit will have the same tree hash as its parent.
    #[clap(long = "empty", short = 'e', conflicts_with = "all")]
    pub empty: bool,

    /// The required commit hash or prefix, in hex.
    ///
    /// [default: the first four hex digits of the commit's tree hash]
    #[clap(long = "prefix", short = 'x', env = "SAVE_COMMIT_PREFIX")]
    pub prefix_hex: Option<String>,

    /// Override the system clock timestamp with a custom one.
    #[clap(long = "timestamp", short = 't', env = "SAVE_TIMESTAMP")]
    pub timestamp: Option<i64>,

    /// Use the next available timestamp after the previous commit, regardless
    /// of the current timestamp.
    ///
    /// If there is no previous commit, this uses the next available timestamp
    /// after the current time (or value provided to `--timestamp`) rounded down
    /// to the closest multiple of `0x1000000` (a period of ~6 months).
    ///
    /// This can be used to help produce deterministic timestamps and commit
    /// IDs for reproducible builds.
    #[clap(long = "timeless", short = '0', env = "SAVE_TIMELESS")]
    pub timeless: bool,

    /// The name to use for the commit's author and committer.
    ///
    /// [default: name from git, or else from parent commit, or else "dev"]
    #[clap(long = "name")]
    pub name: Option<String>,

    /// The email to use for the commit's author and committer.
    ///
    /// [default: email from git, or else from parent commit, or else
    /// "dev@localhost"]
    #[clap(long = "email")]
    pub email: Option<String>,

    /// Prepare the commit, but don't actually update any references in Git.
    #[clap(long, short = 'n')]
    pub dry_run: bool,

    /// Decrease log verbosity. May be used multiple times.
    #[clap(long, short = 'q', parse(from_occurrences), conflicts_with = "verbose")]
    pub quiet: i32,

    /// Override the system clock timestamp with a custom one.
    #[clap(long = "squash", alias = "amend")]
    pub squash: Option<Option<i32>>,

    /// Increase log verbosity. May be used multiple times.
    #[clap(long, short = 'v', parse(from_occurrences), conflicts_with = "quiet")]
    pub verbose: i32,
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
    let repo = open_or_init_repo(&args)?;

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
            } else if args.empty {
                info!("Committing with no changes.");
            } else {
                info!("Nothing to commit (use --empty if this is intentional).");
                return Ok(());
            }
        }
    }

    if !args.dry_run {
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

            if dangerous {
                bail!(
                    "Current directory seems important, refusing to run `git init` automatically."
                );
            } else if empty && !args.empty {
                bail!("Current directory is empty, skipping auto-init (--empty to override).");
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

/// Initialize the typical global environment and parses the typical [Args] for
/// save's [main] CLI entry point.
///
/// # Panics
///
/// This will panic if called multiple times, or if other code attempts
/// conflicting global initialization of systems such as logging.
#[must_use]
pub fn init() -> Args {
    color_eyre::install().unwrap();

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

    tracing_subscriber::util::SubscriberInitExt::init(tracing_subscriber::Layer::with_subscriber(
        tracing_error::ErrorLayer::default(),
        tracing_subscriber::fmt()
            .with_env_filter(::tracing_subscriber::EnvFilter::new(rust_log))
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
