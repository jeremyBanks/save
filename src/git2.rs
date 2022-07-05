//! Extending [`::git2`] (`libgit2`).

#[allow(unused)]
pub(self) use ::git2::{
    Blob, Branch, Commit, Config, Index, Object, ObjectType, Oid, Reference, Remote, Repository,
    Signature, Tag, Time, Tree,
};
use {
    crate::{zigzag::ZugZug, *},
    ::{
        digest::{generic_array::GenericArray, typenum::U20, Digest},
        eyre::{Context, Result},
        itertools::Itertools,
        parking_lot::RwLock,
        petgraph::{
            graphmap::DiGraphMap,
            visit::Topo,
            EdgeDirection::{Incoming, Outgoing},
        },
        std::{
            borrow::Borrow,
            fmt::Debug,
            intrinsics::transmute,
            ops::{Deref, DerefMut},
            path::PathBuf,
        },
        tempfile::TempDir,
    },
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
        let message = "hmm"; // XXX: ???
        let commit = repo.commit(None, &signature, &signature, message, &tree, &[&head])?;
        let commit = repo.find_commit(commit)?;
        Ok(commit)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GraphStats {
    pub revision_index: u32,
    pub generation_index: u32,
    pub commit_index: u32,
}

impl RepositoryExt for Repository {}

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

    /// Testing a different implementation of [`CommitExt::generation_number`].
    #[instrument(level = "debug")]
    #[must_use]
    fn graph_stats(&self) -> GraphStats {
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

        debug!(
            "Loaded commit graph with {} nodes (commits) and {} edges (parent references of \
             commits)",
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

        let commit_index: u32 = (graph.node_count() - 1).try_into().unwrap();
        let generation_index = global_maximum_weight;
        let revision_index = {
            let mut revision_index = 0;
            let mut commit: Commit = self.borrow().clone();
            while let Some(parent) = commit.parents().next() {
                revision_index += 1;
                commit = parent;
            }
            revision_index
        };

        GraphStats {
            revision_index,
            generation_index,
            commit_index,
        }
    }

    // /// Returns a new [`Commit`] with the result of squashing this [`Commit`]
    // /// with its `depth` first-parent ancestors, and any merged-in
    // /// descendant branches.
    // #[instrument(level = "debug")]
    // #[must_use]
    // fn squashed(&self, depth: u32) -> Commit<'repo> {
    //     let commit: &Commit<'repo> = self.borrow();
    //     if depth == 0 {
    //         return commit.clone();
    //     }

    //     let _merged_commits: HashSet<Oid> = [commit.id()].into();

    //     // let mut tail: Commit = commit.clone();
    //     // for _ in 0..depth {
    //     //     let mut first_parent = tail.parents().next().unwrap().clone();
    //     //     merged_commits.insert(first_parent.id());
    //     //     tail = first_parent;

    //     //     // we need to collect all of the non-first parents, and walk all
    // of     //     // their ancestors to see if they're merged in or not
    //     // }

    //     todo!()
    // }

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
        target_mask: Option<&[u8]>,
        min_timestamp: impl Into<Option<i64>>,
        target_timestamp: impl Into<Option<i64>>,
    ) -> Commit<'repo> {
        let target_prefix = target_prefix.to_vec();
        let target_mask = target_mask
            .unwrap_or({
                static DEFAULT: &[u8] = &[0xFF; 20];
                &DEFAULT[..target_prefix.len().min(DEFAULT.len())]
            })
            .iter()
            .copied()
            .collect::<Vec<_>>();
        trace!("Brute forcing a timestamp for {target_prefix:2x?} with mask {target_mask:2x?}");

        let thread_count = num_cpus::get() as u64;
        trace!("Using {thread_count} threads");

        let commit = self.borrow();
        let min_timestamp = min_timestamp
            .into()
            .unwrap_or_else(|| commit.committer().when().seconds());

        let target_timestamp = target_timestamp.into().unwrap_or(min_timestamp);

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

        let best: RwLock<Option<Best>> = RwLock::new(None);
        struct Best {
            index: u64,
            body: String,
            oid: Oid,
            author_timestamp: i64,
            committer_timestamp: i64,
        }

        let target_timestamp = target_timestamp;
        let min_timestamp = min_timestamp;

        let target_mask = &target_mask;
        let target_prefix = &target_prefix;

        std::thread::scope(|scope| {
            let best = &best;
            let mut threads = Vec::new();

            for thread_index in 0..thread_count {
                threads.push(scope.spawn(move || {
                    for local_index in 0u64.. {
                        let index = local_index * thread_count + thread_index;
                        if index % 64 == 0 {
                            if let Some(ref best) = *best.read() {
                                let best_index = best.index;
                                if best_index < index {
                                    trace!("Ending thread {thread_index} as it's past the current-best {best_index}");
                                    break;
                                }
                            }
                        }

                        let (d_author, d_committer) = index.zugzug();

                        let author_timestamp = target_timestamp + d_author;
                        let committer_timestamp = target_timestamp + d_committer;

                        if author_timestamp < min_timestamp {
                            continue;
                        }

                        let candidate_body =
                            commit_create_buffer(author_timestamp, committer_timestamp);

                        let candidate_oid = Oid::for_object("commit", candidate_body.as_ref());

                        if candidate_oid
                            .as_bytes()
                            .iter()
                            .zip(target_prefix.iter())
                            .map(|(a, b)| (a ^ b))
                            .zip(target_mask.iter())
                            .map(|(x, mask)| x & *mask)
                            .all(|x| x == 0)
                        {
                            let mut best = best.write();
                            if best.is_none() || index < best.as_ref().unwrap().index {
                                *best = Some(Best {
                                    index,
                                    author_timestamp,
                                    committer_timestamp,
                                    body: candidate_body,
                                    oid: candidate_oid,
                                });
                            }

                            break;
                        }
                    }
                }));
            }
        });

        let best = best.into_inner().unwrap();

        let brute_forced_commit_oid = commit
            .amend(
                None,
                Signature::new(
                    commit.author().name().unwrap(),
                    commit.author().email().unwrap(),
                    &git2::Time::new(
                        best.author_timestamp,
                        commit.author().when().offset_minutes(),
                    ),
                )
                .as_ref()
                .ok(),
                Signature::new(
                    commit.committer().name().unwrap(),
                    commit.committer().email().unwrap(),
                    &git2::Time::new(
                        best.committer_timestamp,
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
        assert_eq!(best.oid, brute_forced_commit_oid);

        let brute_forced_commit = repo.find_commit(brute_forced_commit_oid).unwrap();
        assert_eq!(best.body.as_bytes(), brute_forced_commit.to_bytes());

        brute_forced_commit
    }
}

impl<'repo> CommitExt<'repo> for Commit<'repo> {}

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

impl OidExt for Oid {}
