#![doc = include_str!("../README.md")]
#![warn(
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::missing_safety_doc
)]

use {
    tracing::debug_span,
    ::{
        clap::Parser,
        digest::Digest,
        eyre::{bail, Result, WrapErr},
        git2::{ErrorCode, Repository, RepositoryInitOptions, Signature, Time},
        std::{env, fs, process::Command},
        tracing::{debug, info, trace, warn},
    },
};

pub mod git;

/// Would you like to SAVE the change?
///
/// Commit everything in the current Git repository, no questions asked.
#[derive(Parser, Debug, Clone)]
#[clap(version)]
#[remain::sorted]
pub struct Args {
    /// Prepare the commit, but don't actually save anything to disk.
    #[clap(long)]
    pub dry_run: bool,

    /// The author email to use for the commit.
    ///
    /// [default: email from git, or else from parent commit, or else "save"]
    #[clap(long, short = 'e')]
    pub email: Option<String>,

    /// The target commit hash or prefix, in hex.
    ///
    /// [default: the commit's tree hash]
    #[clap(long = "hash", short = 'x')]
    pub hash_hex: Option<String>,

    /// Use a manual commit message instead of the default generated message.
    #[clap(long, short = 'm')]
    pub message: Option<String>,

    /// The author name to use for the commit.
    ///
    /// [default: name from git, or else from parent commit, or else "save"]
    #[clap(long, short = 'n')]
    pub name: Option<String>,

    /// The time is NOW.
    ///
    /// [default: the time is ACTUALLY now]
    #[clap(long = "now", short = 'w')]
    pub now_seconds: Option<i64>,

    /// Decrease log verbosity. May be used multiple times.
    #[clap(long, short = 'q', parse(from_occurrences))]
    pub quiet: i32,

    /// Seconds of timestamp allocated for each commit to search.
    #[clap(long="step", short='s', default_value_t = 64 * 2)]
    pub step_seconds: u32,

    /// Increase log verbosity. May be used multiple times.
    #[clap(long, short = 'v', parse(from_occurrences))]
    pub verbose: i32,

    /// Proceed in spite of any warnings.
    #[clap(long, short = 'y')]
    pub yes: bool,
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
pub fn main(args: Args) -> Result<()> {
    trace!("{:#?}", args);

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
                Repository::init_opts(
                    path,
                    RepositoryInitOptions::new()
                        .initial_head("trunk")
                        .no_reinit(true),
                )?
            }
        },
    };

    let head = match repo.head() {
        Ok(head) => Some(head.peel_to_commit().unwrap()),
        Err(err) if err.code() == ErrorCode::UnbornBranch => None,
        Err(err) => {
            bail!("Unexpected error from Git: {:#?}", err);
        },
    };

    let config = repo.config()?;

    let user_name: String = {
        if let Some(args_name) = args.name {
            trace!(
                "Using author name from command line argument: {:?}",
                &args_name
            );
            args_name
        } else if let Ok(config_name) = config.get_string("user.name") {
            debug!(
                "Using author name from Git configuration: {:?}",
                &config_name
            );
            config_name
        } else if let Some(previous_name) = head
            .as_ref()
            .and_then(|x| x.author().name().map(|x| x.to_string()))
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

    let user_email: String = if let Some(args_email) = args.email {
        trace!(
            "Using author email from command line argument: {:?}",
            &args_email
        );
        args_email
    } else if let Ok(config_email) = config.get_string("user.email") {
        debug!(
            "Using author email from Git configuration: {:?}",
            &config_email
        );
        config_email
    } else if let Some(previous_email) = head
        .as_ref()
        .and_then(|x| x.author().email().map(|x| x.to_string()))
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

    let trace_generation = debug_span!("Finding generation index...");
    let generation_index = head
        .as_ref()
        .map(|commit| git::find_generation_index(commit) + 1)
        .unwrap_or(0);
    drop(trace_generation);

    let mut index = repo.index()?;

    index.add_all(["*"], Default::default(), Default::default())?;
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

    let revision_index = generation_index + 1;
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
        seconds = seconds - seconds % snap_seconds
    }

    let parents = &head.iter().collect::<Vec<_>>();

    let min_timestamp = seconds;
    let max_timestamp = seconds + step_seconds - 1;

    let mut target_hash = args
        .hash_hex
        .map(|s| hex::decode(s).wrap_err("target hash must be hex").unwrap())
        .unwrap_or_default();
    target_hash.append(&mut tree.id().as_bytes().to_vec());

    let trace_brute = debug_span!("Finding the most tree-like commit hash within step...");

    // TODO: Do this ourselves, and make it parallel.
    let (_score, author_timestamp, commit_timestamp) =
        ((min_timestamp..=max_timestamp).map(|author_timestamp| {
            (author_timestamp..=max_timestamp)
                .map(|commit_timestamp| {
                    let candidate = repo
                        .commit_create_buffer(
                            &Signature::new(
                                &user_name,
                                &user_email,
                                &Time::new(author_timestamp, offset),
                            )
                            .unwrap(),
                            &Signature::new(
                                &user_name,
                                &user_email,
                                &Time::new(commit_timestamp, offset),
                            )
                            .unwrap(),
                            &message,
                            &tree,
                            parents,
                        )
                        .unwrap()
                        .to_vec();
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

                    (score, author_timestamp, commit_timestamp)
                })
                .min()
                .unwrap()
        }))
        .min()
        .unwrap();
    drop(trace_brute);

    if !args.dry_run {
        repo.commit(
            Some("HEAD"),
            &Signature::new(
                &user_name,
                &user_email,
                &Time::new(author_timestamp, offset),
            )
            .unwrap(),
            &Signature::new(
                &user_name,
                &user_email,
                &Time::new(commit_timestamp, offset),
            )
            .unwrap(),
            &message,
            &tree,
            parents,
        )?;
    } else {
        info!("Skipping commit write because this is a dry run.");
    }

    eprintln!();

    Command::new("git")
        .args(&[
            "--no-pager",
            "log",
            "--name-status",
            "--format=raw",
            "--graph",
            "--decorate",
            "-n",
            "2",
        ])
        .status()?;

    eprintln!();

    Ok(())
}

/// Initialize the typical global environment and parses the typical [Args] for
/// save's [main] CLI entry point.
///
/// # Panics
///
/// This will panic if called multiple times, or if other code attempts
/// conflicting global initialization of systems such as logging.
pub fn init() -> Args {
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
            .without_time()
            .with_span_events(
                tracing_subscriber::fmt::format::FmtSpan::NEW
                    | tracing_subscriber::fmt::format::FmtSpan::CLOSE,
            )
            .finish(),
    ));

    args
}
