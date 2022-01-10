#![doc = include_str!("../README.md")]
#![warn(
    missing_docs,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::missing_safety_doc
)]

use {
    clap::Parser,
    digest::Digest,
    eyre::{bail, Result, WrapErr},
    git2::{
        Commit, ErrorCode, Index, Oid, Repository, RepositoryInitOptions, RepositoryState,
        Signature, Time,
    },
    rayon::prelude::*,
    std::{
        cell::RefCell, cmp::max, collections::HashMap, env, fs, path::PathBuf, process::Command,
        rc::Rc,
    },
    thousands::Separable,
    tracing::{debug, debug_span, info, instrument, trace, warn},
};

/// Would you like to SAVE the change?
///
/// Commit everything in the current Git repository, no questions asked.
#[derive(Parser, Debug, Clone)]
#[clap(version)]
#[remain::sorted]
pub struct Args {
    /// Prepare the commit, but don't actually save anything to disk.
    #[clap(long, short = 'n')]
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

    /// Commit message to use.
    ///
    /// [default: generated from generation number, tree hash, and parents]
    #[clap(long, short = 'm')]
    pub message: Option<String>,

    /// The author name to use for the commit.
    ///
    /// [default: name from git, or else from parent commit, or else "save"]
    #[clap(long, short = 'a')]
    pub name: Option<String>,

    /// The time is NOW.
    ///
    /// [default: the time is ACTUALLY now]
    #[clap(long = "now", short = 'w')]
    pub now_seconds: Option<i64>,

    /// Decrease log verbosity. May be used multiple times.
    #[clap(long, short = 'q', parse(from_occurrences))]
    pub quiet: i32,

    /// Squash/amend previous commit(s), instead of adding a new one.
    ///
    /// By default, `--squash` will behave like `git commit --amend`, only
    /// replacing the most recent commit. However, specifying a larger number
    /// such as `--squash=2` will squash that many recent commits (and any
    /// current changes) into a single commit. If any of those commits are
    /// merges, any non-squashed parents will be added as parents of the
    /// squashed commit.
    #[clap(
        long = "squash",
        aliases = &["amend", "fix"],
        short = 's',
        default_value = "0",
        default_missing_value = "1"
    )]
    pub squash_commits: u32,

    /// Seconds of timestamp allocated for each commit to search.
    ///
    /// The number of possibilities searched is the half the square of this
    /// value.
    #[clap(long="step", short='t', default_value_t = 64 * 2)]
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
#[instrument(level = "debug", skip(args))]
pub fn main(args: Args) -> Result<()> {
    if args.squash_commits > 0 {
        todo!("--squash has not been implemented");
    }

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
        .map(|commit| generation_number(commit) + 1)
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

    let base_commit = repo
        .commit_create_buffer(
            &Signature::new(&user_name, &user_email, &Time::new(min_timestamp, offset)).unwrap(),
            &Signature::new(&user_name, &user_email, &Time::new(min_timestamp, offset)).unwrap(),
            &message,
            &tree,
            parents,
        )
        .unwrap()
        .to_vec();
    let base_commit = std::str::from_utf8(&base_commit)
        .wrap_err("commit must be valid utf-8")
        .unwrap();

    let (author_timestamp, commit_timestamp) =
        brute_force_timestamps(base_commit, &target_hash, min_timestamp, max_timestamp);

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

    Ok((user_name, user_email))
}

/// Generates an updated [git2::Index] with every file in the directory.
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

/// Opens or initializes a new repository in CWD or GIT_DIR, if args allow it.
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

/// Brute forces timestamps for a Git commit.
///
/// Given a raw Git commit as a string, finds the timestamps in the given range
/// that will produce the closest commit ID to target_hash. We ensure that
/// min_timestamp <= author_timestamp <= committer_timestamp <= max_timestamp
/// because it would be weird to it committed before being authored.
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
        .unwrap();
    let author_line_pieces = &base_commit_lines[author_line_index]
        .split(' ')
        .collect::<Vec<_>>();
    let committer_line_index = base_commit_lines
        .iter()
        .position(|line| line.starts_with("committer "))
        .unwrap();
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

/// Finds the generation number of a given Git commit.
///
/// The generation index is the number of edges of the longest path between the
/// given commit and an initial commit (one with no parents, which has an
/// implicit generation index of 0). The Git documentation also refers to this
/// as the "topological level" of a commit (https://git-scm.com/docs/commit-graph).
#[instrument(level = "debug")]
pub fn generation_number(commit: &Commit) -> u32 {
    let head = commit.clone();

    #[derive(Debug, Clone)]
    struct CommitNode {
        // number of edges (git children) whose distances hasn't been accounted-for yet
        unaccounted_edges_in: u32,
        // max distance from head of accounted-for edges
        max_distance_from_head: u32,
        // git parents of this node
        edges_out: Vec<Rc<RefCell<CommitNode>>>,
    }

    let (root, _leaves) = {
        let span = debug_span!("load_git_graph");
        let _guard = span.enter();

        let mut all_commits = HashMap::<Oid, Rc<RefCell<CommitNode>>>::new();
        let mut initial_commits = vec![];

        #[derive(Debug, Clone)]
        struct CommitWalking<'repo> {
            commit: Commit<'repo>,
            from: Option<Rc<RefCell<CommitNode>>>,
        }

        let mut walks = vec![CommitWalking {
            commit: head.clone(),
            from: None,
        }];

        while let Some(CommitWalking { commit, from }) = walks.pop() {
            let from = &from;
            all_commits
                .entry(commit.id())
                .and_modify(|node| {
                    if let Some(from) = from {
                        from.borrow_mut().edges_out.push(node.clone());
                        node.borrow_mut().unaccounted_edges_in += 1;
                    }
                })
                .or_insert_with(|| {
                    let node = Rc::new(RefCell::new(CommitNode {
                        edges_out: vec![],
                        unaccounted_edges_in: 1,
                        max_distance_from_head: 0,
                    }));

                    if let Some(from) = from {
                        from.borrow_mut().edges_out.push(node.clone());
                    }

                    if commit.parents().len() == 0 {
                        debug!("Found an initial commit: {:?}", commit);
                        initial_commits.push(node.clone());
                    } else {
                        for parent in commit.parents() {
                            walks.push(CommitWalking {
                                commit: parent,
                                from: Some(node.clone()),
                            });
                        }
                    }

                    node
                });
        }

        info!(
            "Loaded {} commits, containing {} initial commits.",
            all_commits.len().separate_with_underscores(),
            initial_commits.len(),
        );

        let head = all_commits.get(&head.id()).unwrap().clone();
        (head, initial_commits)
    };

    let generation_number = {
        let span = debug_span!("measure_git_graph");
        let _guard = span.enter();

        let mut generation_number = 0;

        let mut live = vec![root];

        while let Some(commit) = live.pop() {
            let commit = commit.borrow_mut();

            if commit.edges_out.is_empty() {
                generation_number = max(generation_number, commit.max_distance_from_head);
            } else {
                for parent in commit.edges_out.iter() {
                    let mut parent_mut = parent.borrow_mut();
                    parent_mut.max_distance_from_head = max(
                        parent_mut.max_distance_from_head,
                        commit.max_distance_from_head + 1,
                    );
                    parent_mut.unaccounted_edges_in -= 1;

                    if parent_mut.unaccounted_edges_in == 0 {
                        live.push(parent.clone());
                    }
                }
            }
        }

        generation_number
    };

    generation_number
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
