//! Helpers for [`::git2`].

#[allow(unused)]
pub(self) use git2::{
    Blob, Branch, Commit, Config, Index, Object, ObjectType, Oid, Reference, Remote, Repository,
    Signature, Tag, Time, Tree,
};
use {
    digest::{generic_array::GenericArray, Digest},
    eyre::{Context, Result},
    itertools::Itertools,
    petgraph::{
        graphmap::DiGraphMap,
        visit::Topo,
        EdgeDirection::{Incoming, Outgoing},
    },
    rayon::iter::{IntoParallelIterator, ParallelIterator},
    std::{
        borrow::Borrow,
        cell::RefCell,
        cmp::max,
        collections::{HashMap, HashSet},
        fmt::Debug,
        intrinsics::transmute,
        ops::{Deref, DerefMut},
        path::PathBuf,
        rc::Rc,
    },
    tempfile::TempDir,
    thousands::Separable,
    tracing::{debug, debug_span, info, instrument, trace, warn},
    typenum::U20,
};

/// Extension methods for [`Repository`].
pub trait RepositoryExt: Borrow<Repository> {
    /// Returns a Index with the current contents of the repository's working
    /// tree, as though everything inside of it had been committed on top of
    /// the current head. Submodules are skipped with a warning logged.
    ///
    /// These changes can be written back to the repository index on disk with
    /// [`Index::write`], or converted into a [`Tree`] with
    /// [`Index::write_tree`].
    ///
    /// # Panics
    ///
    /// If the repository is bare (per [`Repository::is_bare`]).
    #[instrument(level = "debug", skip_all)]
    #[must_use]
    fn working_index(&self) -> Result<Index> {
        let repo: &Repository = self.borrow();

        if repo.is_bare() {
            panic!("Repository is bare!");
        }

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

    /// Creates a [`Repository`] backed by a new temporary directory.
    #[instrument(level = "debug", skip_all)]
    #[must_use]
    fn temporary() -> Result<TemporaryRepository> {
        let dir = TempDir::new()?;
        let repo = Repository::init(&dir)?;

        Ok(TemporaryRepository { repo, dir })
    }

    /// Returns a signature for use in the current repository.
    ///
    /// Defaults to the `user.name` and `user.email` configured in Git. If
    /// these are not present, a warning is logged and we fall back to the
    /// author of the current HEAD commit. If there *is* no HEAD commit, we
    /// fall back to a generic placeholder signature.
    fn signature_or_fallback(&self) -> Signature {
        let _default_name = "dev";
        let _default_email = "dev@localhost";

        let repo: &Repository = self.borrow();
        let _signature = repo.signature();

        todo!()
    }

    /// Saves all changes in the working directory to this repository using
    /// insensible defaults.
    ///
    /// # Errors
    ///
    /// ?
    fn save(&self) -> Result<Commit> {
        let repo: &Repository = self.borrow();

        let mut index = self.working_index()?;
        let tree = index.write_tree()?;
        let tree = repo.find_tree(tree)?;
        let head = repo.head()?.peel_to_commit()?;
        let signature = repo.signature_or_fallback();
        let message = "hmm";
        let commit = repo.commit(None, &signature, &signature, message, &tree, &[&head])?;
        let commit = repo.find_commit(commit)?;
        Ok(commit)
    }
}

impl<T> RepositoryExt for T where T: Borrow<Repository> {}

/// A [`Repository`] in a temporary directory.
///
/// Because the backing directory for the repository will be deleted when this
/// struct is [`Drop`]ped, we don't provide any way to move the [`Repository`]
/// out, just deref to it, but we can't stop users from making a mess if they
/// clone it.
///
/// # Panic Safety
///
/// If this isn't dropped, the temporary directory will not be deleted. See
/// [`TempDir`]'s docs for more information.
#[must_use]
pub struct TemporaryRepository {
    repo: Repository,
    #[allow(unused)]
    dir: TempDir,
}

impl Debug for TemporaryRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TemporaryRepository {{ at {:?} }}", self.repo.path())
    }
}

impl Deref for TemporaryRepository {
    type Target = Repository;

    fn deref(&self) -> &Repository {
        &self.repo
    }
}

impl DerefMut for TemporaryRepository {
    fn deref_mut(&mut self) -> &mut Repository {
        &mut self.repo
    }
}

/// Extension methods for [`Commit`].
pub trait CommitExt<'repo>: Borrow<Commit<'repo>> + Debug {
    /// Returns the raw contents of the underlying Git commit object.
    ///
    /// This is similar to [`Repository::commit_create_buffer`], but for
    /// an existing [`Commit`].
    fn to_bytes(&self) -> Vec<u8> {
        let commit: &Commit = self.borrow();
        let header_bytes = commit.raw_header_bytes();
        let message_bytes_raw = commit.message_raw_bytes();

        let mut body = Vec::with_capacity(header_bytes.len() + 1 + message_bytes_raw.len());
        body.extend(header_bytes);
        body.push(b'\n');
        body.extend(message_bytes_raw);

        if cfg!(debug_assertions) {
            let digest = Oid::for_object("commit", &body);
            let id = commit.id();
            assert_eq!(
                digest, id,
                "to_bytes produced a commit object with the wrong hash"
            );
        }

        body
    }

    /// Finds the generation number of this commit.
    ///
    /// The generation index is the number of edges of the longest path between
    /// the given commit and an initial commit (one with no parents, which
    /// has an implicit generation index of 0). The Git documentation also
    /// refers to this as the "topological level" of a commit
    /// (<https://git-scm.com/docs/commit-graph>).
    #[instrument(level = "debug")]
    #[must_use]
    fn generation_number(&self) -> u32 {
        let commit: &Commit = self.borrow();
        let head = commit.clone();

        #[derive(Debug, Clone)]
        struct CommitNode {
            /// Number of edges (Git children) whose distances hasn't been
            /// accounted-for yet.
            unaccounted_edges_in: u32,
            /// Max distance from head of accounted-for edges.
            max_distance_from_head: u32,
            /// Git parents of this node.
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
                            from.borrow_mut().edges_out.push(Rc::clone(node));
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
                            from.borrow_mut().edges_out.push(Rc::clone(&node));
                        }

                        if commit.parents().len() == 0 {
                            debug!("Found an initial commit: {:?}", commit);
                            initial_commits.push(Rc::clone(&node));
                        } else {
                            for parent in commit.parents() {
                                walks.push(CommitWalking {
                                    commit: parent,
                                    from: Some(Rc::clone(&node)),
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

            let head = Rc::clone(all_commits.get(&head.id()).unwrap());
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
                            live.push(Rc::clone(parent));
                        }
                    }
                }
            }

            generation_number
        };

        generation_number
    }

    /// Testing a different implementation of [`CommitExt::generation_number`].
    #[instrument(level = "debug")]
    #[must_use]
    fn generation_number_via_petgraph(&self) -> u32 {
        let commit: &Commit = self.borrow();

        // Git commit graph as petgraph:
        // - nodes are the commit Oids
        // - edges are directed from children to parent commits
        // - edges "weights" are to be their distance from head, starting with 0
        let mut graph = DiGraphMap::<Oid, u32>::new();

        let mut heads: Vec<Commit> = vec![commit.clone()];
        while !heads.is_empty() {
            let head = heads.pop().unwrap();
            let oid = head.id();

            if graph.edges_directed(oid, Outgoing).count() > 0 {
                // This has already been walked.
                // If there are no edges, this either hasn't been walked,
                // or it's a root node, which we can harmlessly process
                // again.
                continue;
            }

            for parent in head.parents() {
                graph.add_edge(oid, parent.id(), 0);
                heads.push(parent.clone());
            }
        }

        info!(
            "Constructed graph with {} nodes and {} edges",
            graph.node_count(),
            graph.edge_count()
        );

        let mut visitor = Topo::new(&graph);
        let mut global_maximum_weight = 0;
        while let Some(node) = visitor.next(&graph) {
            let max_incoming_weight = graph
                .edges_directed(node, Incoming)
                .map(|(_, _, weight)| *weight)
                .max()
                .unwrap_or_default();
            let outgoing_weight = max_incoming_weight + 1;

            let parents = graph
                .edges_directed(node, Outgoing)
                .map(|(a, b, w)| (a, b, *w))
                .collect_vec();
            for (a, b, existing_weight) in parents {
                if outgoing_weight > existing_weight {
                    graph[(a, b)] = outgoing_weight;
                    if outgoing_weight > global_maximum_weight {
                        global_maximum_weight = outgoing_weight;
                    }
                }
            }
        }

        global_maximum_weight
    }

    /// Returns a new [`Commit`] with the result of squashing this [`Commit`]
    /// with its `depth` first-parent ancestors, and any merged-in
    /// descendant branches.
    #[instrument(level = "debug")]
    #[must_use]
    fn squashed(&self, depth: u32) -> Commit<'repo> {
        let commit: &Commit<'repo> = self.borrow();
        if depth == 0 {
            return commit.clone();
        }

        let mut merged_commits: HashSet<Oid> = [commit.id()].into();

        let mut tail: Commit = commit.clone();
        for _ in 0..depth {
            let mut first_parent = tail.parents().next().unwrap().clone();
            merged_commits.insert(first_parent.id());
            tail = first_parent;

            // we need to collect all of the non-first parents, and walk all of
            // their ancestors to see if they're merged in or not
        }

        todo!()
    }

    /// Modifies the committer and author timestamps on a commit to produce a
    /// commit ID as close as possible to a given target, within a timestamp
    /// range.
    ///
    /// The "committer" timestamp will always be following or concurrent-with
    /// the "author" timestamp, so this searches half the square of the number
    /// of possible timestamps in the range. If multiple complete matches for
    /// the prefix exist within the time span, this function will return the
    /// one with the lowest committer timestamp, and if that's a tie it will
    /// use the one with the lowest author timestamp.
    ///
    /// If `min_timestamp` is not specified, it will default to the current
    /// committer timestamp in the commit.
    ///
    /// If `max_timestamp` is not specified, this will continue searching until
    /// it has a full match for target commit ID prefix.
    ///
    /// # Panics
    ///
    /// If `min_timestamp` > `max_timestamp`.
    #[instrument(level = "debug", skip_all)]
    #[must_use]
    fn brute_force_timestamps(
        &self,
        repo: &'repo Repository,
        target_prefix: &[u8],
        min_timestamp: impl Into<Option<i64>>,
        max_timestamp: impl Into<Option<i64>>,
    ) -> BruteForcedCommit<'repo> {
        let commit = self.borrow();
        let min_timestamp = min_timestamp
            .into()
            .unwrap_or_else(|| commit.author().when().seconds());

        // TODO: actually short-circuit on full matches so this isn't always an infinite
        // loop
        let max_timestamp = max_timestamp.into().unwrap_or(i64::MAX);

        let base_commit = String::from_utf8(self.to_bytes()).unwrap();

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

        let (_best_score, best_committer_timestamp, best_author_timestamp, best_oid, best_body) =
            ((min_timestamp..=max_timestamp)
                .into_par_iter()
                .map(|author_timestamp| {
                    (author_timestamp..=max_timestamp)
                        .into_par_iter()
                        .map(|committer_timestamp| {
                            let candidate_body =
                                commit_create_buffer(author_timestamp, committer_timestamp);
                            let candidate_oid = Oid::for_object("commit", candidate_body.as_ref());

                            let score = candidate_oid
                                .as_bytes()
                                .iter()
                                .zip(target_prefix.iter())
                                .map(|(a, b)| (a ^ b))
                                .collect::<Vec<u8>>();

                            (
                                score,
                                committer_timestamp,
                                author_timestamp,
                                candidate_oid,
                                candidate_body,
                            )
                        })
                        .min()
                        .unwrap()
                }))
            .min()
            .unwrap();

        let brute_forced_commit_oid = commit
            .amend(
                None,
                Signature::new(
                    commit.author().name().unwrap(),
                    commit.author().email().unwrap(),
                    &git2::Time::new(
                        best_author_timestamp,
                        commit.author().when().offset_minutes(),
                    ),
                )
                .as_ref()
                .ok(),
                Signature::new(
                    commit.committer().name().unwrap(),
                    commit.committer().email().unwrap(),
                    &git2::Time::new(
                        best_committer_timestamp,
                        commit.committer().when().offset_minutes(),
                    ),
                )
                .as_ref()
                .ok(),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(best_oid, brute_forced_commit_oid);

        let brute_forced_commit = repo.find_commit(brute_forced_commit_oid).unwrap();
        assert_eq!(best_body.as_bytes(), brute_forced_commit.to_bytes());

        if best_oid.as_bytes().starts_with(target_prefix) {
            debug!("Brute-forced a complete prefix match: {best_oid} for {target_prefix:02x?}");
            BruteForcedCommit::Complete {
                commit: brute_forced_commit,
            }
        } else {
            debug!("Brute-forced a partial prefix match: {best_oid} for {target_prefix:02x?}");
            BruteForcedCommit::Incomplete {
                commit: brute_forced_commit,
            }
        }
    }
}

impl<'repo, T> CommitExt<'repo> for T where T: Borrow<Commit<'repo>> + Debug {}

/// The commit resulting from a [`CommitExt::brute_force_timestamps`] call,
/// wrapped to indicate whether the target prefix was complete or incompletely
/// matched.
#[derive(Debug, Clone)]
#[must_use]
pub enum BruteForcedCommit<'repo> {
    /// The specified `target_prefix` was entirely matched.
    Complete {
        /// The resulting commit.
        commit: Commit<'repo>,
    },
    /// The specified `target_prefix` was not entirely matched.
    Incomplete {
        /// The resulting commit.
        commit: Commit<'repo>,
    },
}

impl<'repo> Borrow<Commit<'repo>> for BruteForcedCommit<'repo> {
    fn borrow(&self) -> &Commit<'repo> {
        self.commit()
    }
}

impl<'repo> From<BruteForcedCommit<'repo>> for Commit<'repo> {
    fn from(commit: BruteForcedCommit<'repo>) -> Self {
        match commit {
            BruteForcedCommit::Complete { commit, .. }
            | BruteForcedCommit::Incomplete { commit, .. } => commit,
        }
    }
}

impl<'repo> BruteForcedCommit<'repo> {
    /// Returns a reference to the underlying [`Commit`].
    #[must_use]
    pub const fn commit(&self) -> &Commit<'repo> {
        match self {
            BruteForcedCommit::Complete { commit, .. }
            | BruteForcedCommit::Incomplete { commit, .. } => commit,
        }
    }

    /// Returns a reference to the underlying [`Commit`] if it is a complete
    /// match.
    #[must_use]
    pub const fn complete(&self) -> Option<&Commit<'repo>> {
        match self {
            BruteForcedCommit::Complete { commit, .. } => Some(commit),
            _ => None,
        }
    }

    /// Returns a reference to the underlying [`Commit`] if it is not a complete
    /// match.
    #[must_use]
    pub const fn incomplete(&self) -> Option<&Commit<'repo>> {
        match self {
            BruteForcedCommit::Incomplete { commit, .. } => Some(commit),
            _ => None,
        }
    }
}

/// Extension methods for [`Oid`].
pub trait OidExt: Borrow<Oid> + Debug {
    /// This is similar to [`Oid::from_bytes`], but faster.
    #[allow(unsafe_code)]
    #[must_use]
    fn from_array(bytes: [u8; 20]) -> Oid {
        // An `Oid` is a simple data type with the same internal representation
        // as a `[u8; 20]` internally. However, all of the public interfaces
        // for creating an `Oid` have some amount of unnecessary overhead.
        let oid: Oid = unsafe { transmute(bytes) };
        if cfg!(debug_assertions) {
            // cross-check with git2
            let expected = Oid::from_bytes(&bytes).unwrap();
            assert_eq!(expected, oid);
        }
        oid
    }

    /// This is similar to [`Oid::hash_object`], but potentially faster.
    #[must_use]
    fn for_object(object_type: &'static str, body: &[u8]) -> Oid {
        let oid: GenericArray<u8, U20> = sha1::Sha1::new()
            .chain_update(object_type)
            .chain_update(" ")
            .chain_update(body.len().to_string())
            .chain_update([0x00])
            .chain_update(&body)
            .finalize();
        let oid: [u8; 20] = oid.into();
        let oid = Oid::from_array(oid);
        if cfg!(debug_assertions) {
            // cross-check with git2
            let expected =
                Oid::hash_object(ObjectType::from_str(object_type).unwrap(), body).unwrap();
            assert_eq!(expected, oid);
        }
        oid
    }
}

impl<T> OidExt for T where T: Borrow<Oid> + Debug {}
