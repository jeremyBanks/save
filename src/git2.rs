//! Helpers for [`::git2`].

#[allow(unused)]
pub(self) use git2::{
    Blob, Branch, Commit, Config, Index, Object, ObjectType, Oid, Reference, Remote, Repository,
    Signature, Tag, Tree,
};
use {
    digest::{consts::U20, generic_array::GenericArray, Digest},
    eyre::{Context, Result},
    std::{
        borrow::Borrow, cell::RefCell, cmp::max, collections::HashMap, fmt::Debug,
        intrinsics::transmute, path::PathBuf, rc::Rc,
    },
    thousands::Separable,
    tracing::{debug, debug_span, info, instrument, trace, warn},
};

/// Extension methods for [`Repository`].
///
/// These methods are all non-destructive: although new objects may be written
/// to the local Git database, nothing will be modified to point to them, nor
/// will the index or working tree be modified.
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
}

impl<T> RepositoryExt for T where T: Borrow<Repository> {}

/// Extension methods for [`Commit`].
///
/// These methods are all non-destructive: although new objects may be written
/// to the local Git database, nothing will be modified to point to them, nor
/// will the index or working tree be modified.
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
    /// refers to this as the "topological level" of a commit (<https://git-scm.com/docs/commit-graph>).
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

    /// Returns a new Commit with the result of squashing this commit with it
    /// `depth` first-parent ancestors, and any merged-in descendant
    /// branches.
    #[instrument(level = "debug")]
    #[must_use]
    fn squashed(&self, depth: u32) -> Commit<'repo> {
        let commit: &Commit<'repo> = self.borrow();
        if depth == 0 {
            return commit.clone();
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
        target_prefix: &[u8],
        min_timestamp: impl Into<Option<i64>>,
        max_timestamp: impl Into<Option<i64>>,
    ) -> BruteForcedCommit<'repo> {
        let commit = self.borrow();
        let min_timestamp = min_timestamp
            .into()
            .unwrap_or_else(|| commit.author().when().seconds());
        let max_timestamp = max_timestamp.into().unwrap_or(i64::MAX);

        if target_prefix.is_empty() {
            return BruteForcedCommit::Complete {
                commit: commit.clone(),
            };
        }

        let _ = (min_timestamp, max_timestamp);

        todo!("create a commit")
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
        /// The number of leading bits of the commit ID that match the target.
        matched_bits: u8,
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
            BruteForcedCommit::Complete { commit }
            | BruteForcedCommit::Incomplete { commit, .. } => commit,
        }
    }
}

impl<'repo> BruteForcedCommit<'repo> {
    /// Returns a reference to the underlying [`Commit`].
    #[must_use]
    pub const fn commit(&self) -> &Commit<'repo> {
        match self {
            BruteForcedCommit::Complete { commit }
            | BruteForcedCommit::Incomplete { commit, .. } => commit,
        }
    }

    /// Returns a reference to the underlying [`Commit`] if it is a complete
    /// match.
    #[must_use]
    pub const fn complete(&self) -> Option<&Commit<'repo>> {
        match self {
            BruteForcedCommit::Complete { commit } => Some(commit),
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
    /// This is similar to [`Oid::from_bytes`], but *potentially* a bit faster.
    #[allow(unsafe_code)]
    #[must_use]
    fn from_array(oid: [u8; 20]) -> Oid {
        // An `Oid` is a simple data type with the same internal representation
        // as a `[u8; 20]` internally. However, all of the external interfaces
        // for creating an `Oid` have some amount of unnecessary overhead.
        unsafe { transmute(oid) }
    }

    /// This is similar to [`Oid::hash_object`], but *potentially* a bit faster.
    #[must_use]
    fn for_object(object_type: &'static str, body: &[u8]) -> Oid {
        let oid: GenericArray<u8, U20> = sha1::Sha1::new()
            .chain_update(object_type)
            .chain_update(" ")
            .chain_update(body.len().to_string())
            .chain_update([0x00])
            .chain_update(&body)
            .finalize();
        Oid::from_array(oid.into())
    }
}

impl<T> OidExt for T where T: Borrow<Oid> + Debug {}
