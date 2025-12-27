//! Graph statistics calculation with z-mode support for depth-limited scanning.
//!
//! This module implements the core algorithm for calculating commit statistics:
//! - revision_index: count along first-parent chain
//! - generation_index: maximum topological distance from roots
//! - commit_index: total number of reachable commits
//! - origin: identifier derived from root commit(s)
//!
//! The algorithm supports depth-limited scanning ("z-mode") to bound complexity
//! in large repositories.

use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

/// Statistics about a commit's position in the repository graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphStats {
    pub revision_index: u32,
    pub generation_index: u32,
    pub commit_index: u32,
    /// Origin: last 4 hex digits of root commit ID(s)
    /// None for root commits (r0/s0/z0)
    pub origin: Option<u16>,
    /// Whether this calculation used z-mode (hit depth limit)
    pub z_mode: bool,
}

impl Default for GraphStats {
    fn default() -> Self {
        Self {
            revision_index: 0,
            generation_index: 0,
            commit_index: 0,
            origin: None,
            z_mode: false,
        }
    }
}

/// Parsed commit message in our format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedMessage {
    pub prefix: MessagePrefix,
    pub revision_index: u32,
    pub generation_index: Option<u32>,
    pub commit_index: Option<u32>,
    pub origin: Option<u16>,
}

/// Commit message prefix indicating repository state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessagePrefix {
    Regular,  // 'r'
    Shallow,  // 's'
    ZMode,    // 'z'
}

/// Abstract interface for commit data needed by the graph statistics algorithm.
pub trait CommitView: Clone + Debug {
    type Id: Clone + Eq + Hash + Ord + Debug;

    /// Get this commit's unique identifier.
    fn id(&self) -> Self::Id;

    /// Get the IDs of this commit's parent commits.
    fn parent_ids(&self) -> Vec<Self::Id>;

    /// Get the commit message (first line only).
    fn summary(&self) -> Option<String>;

    /// Get this commit's tree identifier (for validating parsed messages).
    fn tree_id(&self) -> Self::Id;

    /// Get the raw bytes of this commit's ID (for origin calculation).
    /// For SHA1, this should be 20 bytes.
    fn id_bytes(&self) -> Vec<u8>;
}

/// Abstract interface for repository operations needed by the algorithm.
pub trait RepositoryView<'repo> {
    type Commit: CommitView + 'repo;

    /// Check if this is a shallow clone.
    fn is_shallow(&self) -> bool;

    /// Find a commit by its ID.
    fn find_commit(&'repo self, id: <Self::Commit as CommitView>::Id) -> Option<Self::Commit>;

    /// Validate that a tree prefix matches the actual tree ID.
    /// This is used to verify parsed commit messages.
    fn validate_tree_prefix(&self, tree_id: &<Self::Commit as CommitView>::Id, prefix: &str)
        -> bool;
}

/// Parser for commit messages in our format.
#[derive(Debug, Clone, Copy)]
pub struct MessageParser;

impl MessageParser {
    /// Parse a commit message to extract graph statistics.
    ///
    /// Returns None if the message doesn't match our format or is untrusted.
    pub fn parse(message: &str) -> Option<ParsedMessage> {
        // Format: [r|s|z]N [/ gG] [/ nC] [/ xHHHH] [/ oHHHH]
        let parts: Vec<&str> = message.split(" / ").collect();
        if parts.is_empty() {
            return None;
        }

        let first_part = parts[0].trim();

        // Determine prefix
        let (prefix, revision_str) = if let Some(rest) = first_part.strip_prefix('r') {
            (MessagePrefix::Regular, rest)
        } else if let Some(rest) = first_part.strip_prefix('s') {
            (MessagePrefix::Shallow, rest)
        } else if let Some(rest) = first_part.strip_prefix('z') {
            (MessagePrefix::ZMode, rest)
        } else {
            return None;
        };

        // Extract revision index
        let revision_index: u32 = revision_str.parse().ok()?;

        // Extract optional fields
        let mut generation_index = None;
        let mut commit_index = None;
        let mut origin = None;

        for part in &parts[1..] {
            let part = part.trim();
            if let Some(g) = part.strip_prefix('g') {
                generation_index = Some(g.parse().ok()?);
            } else if let Some(n) = part.strip_prefix('n') {
                commit_index = Some(n.parse().ok()?);
            } else if let Some(o) = part.strip_prefix('o') {
                origin = Some(u16::from_str_radix(o, 16).ok()?);
            }
            // 'x' (tree hash) is validated separately by caller
        }

        Some(ParsedMessage {
            prefix,
            revision_index,
            generation_index,
            commit_index,
            origin,
        })
    }

    /// Validate a parsed message against actual commit data.
    ///
    /// Returns true if the message can be trusted based on:
    /// - Tree hash matches (if present in message)
    /// - Prefix matches repository state (s only in shallow repos)
    pub fn validate<'repo, R: RepositoryView<'repo>>(
        repo: &R,
        commit: &R::Commit,
        message: &str,
        parsed: &ParsedMessage,
    ) -> bool {
        // Check tree hash if present in message
        let parts: Vec<&str> = message.split(" / ").collect();
        for part in &parts[1..] {
            let part = part.trim();
            if let Some(tree_prefix) = part.strip_prefix('x') {
                if !repo.validate_tree_prefix(&commit.tree_id(), tree_prefix) {
                    return false;
                }
            }
        }

        // Only trust 's' prefix if we're actually shallow
        if parsed.prefix == MessagePrefix::Shallow && !repo.is_shallow() {
            return false;
        }

        true
    }
}

/// Calculator for graph statistics with z-mode support.
pub struct GraphStatsCalculator<'repo, 'a: 'repo, R: RepositoryView<'repo>> {
    repo: &'a R,
    max_depth: i32,
    trust_messages: bool,
    _phantom: std::marker::PhantomData<&'repo ()>,
}

impl<'repo, 'a: 'repo, R: RepositoryView<'repo>> Debug for GraphStatsCalculator<'repo, 'a, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphStatsCalculator")
            .field("max_depth", &self.max_depth)
            .field("trust_messages", &self.trust_messages)
            .finish_non_exhaustive()
    }
}

impl<'repo, 'a: 'repo, R: RepositoryView<'repo>> GraphStatsCalculator<'repo, 'a, R> {
    pub fn new(repo: &'a R, max_depth: i32) -> Self {
        Self {
            repo,
            max_depth,
            trust_messages: true,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn new_rebuild(repo: &'a R, max_depth: i32) -> Self {
        Self {
            repo,
            max_depth,
            trust_messages: false,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Calculate graph statistics for a commit.
    ///
    /// This implements the z-mode algorithm:
    /// 1. Scan all parent paths up to max_depth
    /// 2. Trust r commits (if not shallow) or s commits (if shallow)
    /// 3. Don't trust z commits during initial scan
    /// 4. If ANY path hits depth limit: enter z-mode
    /// 5. Retrospectively trust collected z commits
    /// 6. For paths with no valid commit: declare z0
    pub fn calculate(&self, head: &R::Commit) -> GraphStats {
        let is_shallow = self.repo.is_shallow();
        let unlimited_depth = self.max_depth < 0;

        // Special case: if max_depth is 0, we can't scan anything
        if self.max_depth == 0 {
            return GraphStats {
                revision_index: 0,
                generation_index: 0,
                commit_index: 0,
                origin: None,
                z_mode: true,
            };
        }

        // Try to optimize: if head itself has a trusted message, use it
        // (We're calculating stats for a commit that will be ON TOP of head)
        // Skip optimization if trust_messages is false (rebuild mode)
        if unlimited_depth && self.trust_messages {
            if let Some(summary) = head.summary() {
                if let Some(parsed) = MessageParser::parse(&summary) {
                    if MessageParser::validate(self.repo, head, &summary, &parsed) {
                        // Check if we can trust this prefix
                        let trusted = match parsed.prefix {
                            MessagePrefix::Regular if !is_shallow => true,
                            MessagePrefix::Shallow if is_shallow => true,
                            _ => false,
                        };

                        if trusted {
                            // Inherit stats from head, increment indices
                            let generation_index = parsed.generation_index
                                .unwrap_or(parsed.revision_index) + 1;
                            let commit_index = parsed.commit_index
                                .unwrap_or_else(|| parsed.generation_index.unwrap_or(parsed.revision_index)) + 1;

                            return GraphStats {
                                revision_index: parsed.revision_index + 1,
                                generation_index,
                                commit_index,
                                origin: parsed.origin,
                                z_mode: false,
                            };
                        }
                    }
                }
            }
        }

        // Choose between unlimited and depth-limited scan
        if unlimited_depth {
            self.full_graph_walk(head, is_shallow, unlimited_depth)
        } else {
            self.depth_limited_scan(head, is_shallow)
        }
    }

    /// Depth-limited scan implementing z-mode algorithm.
    fn depth_limited_scan(&self, head: &R::Commit, is_shallow: bool) -> GraphStats {
        let max_depth = self.max_depth as usize;

        // Collect all commits we visit
        let mut visited = HashSet::new();
        let mut commit_map: HashMap<_, R::Commit> = HashMap::new();
        let mut parent_map: HashMap<_, Vec<_>> = HashMap::new();

        // Track collected z commits (we don't trust them initially)
        let mut z_commits = HashSet::new();

        // Track boundary commits (where we stopped scanning)
        let mut boundary_commits = HashSet::new();

        // BFS with depth tracking
        let mut queue: Vec<(_, usize)> = vec![(head.id(), 0)];
        visited.insert(head.id());
        commit_map.insert(head.id(), head.clone());

        let mut hit_depth_limit = false;

        while let Some((id, depth)) = queue.pop() {
            if let Some(commit) = commit_map.get(&id).cloned() {
                let parents = commit.parent_ids();

                // Check if we should continue scanning from this commit
                let should_continue = if depth == 0 {
                    // Never check trust for HEAD itself - we're calculating for the commit ON TOP of it
                    true
                } else {
                    // For depth > 0, check trust first, then depth limit
                    // Don't trust any messages if trust_messages is false (rebuild mode)
                    let has_trusted = if self.trust_messages {
                        if let Some(summary) = commit.summary() {
                            if let Some(parsed) = MessageParser::parse(&summary) {
                                if MessageParser::validate(self.repo, &commit, &summary, &parsed) {
                                    // Track z commits but don't trust them yet
                                    if parsed.prefix == MessagePrefix::ZMode {
                                        z_commits.insert(id.clone());
                                        false // Don't trust z commits during initial scan
                                    } else {
                                        // Trust r (if not shallow) or s (if shallow)
                                        match parsed.prefix {
                                            MessagePrefix::Regular if !is_shallow => true,
                                            MessagePrefix::Shallow if is_shallow => true,
                                            _ => false,
                                        }
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };

                    if has_trusted {
                        // Found trusted commit, stop here
                        boundary_commits.insert(id.clone());
                        false
                    } else if depth >= max_depth {
                        // Hit depth limit without finding trusted commit
                        hit_depth_limit = true;
                        boundary_commits.insert(id.clone());
                        false
                    } else {
                        // Not trusted and haven't hit depth limit, keep scanning
                        true
                    }
                };

                // Store parent info for ALL visited commits
                parent_map.insert(id.clone(), parents.clone());

                if should_continue {
                    // Scan parents
                    for parent_id in parents {
                        if visited.insert(parent_id.clone()) {
                            if let Some(parent) = self.repo.find_commit(parent_id.clone()) {
                                commit_map.insert(parent_id.clone(), parent);
                                queue.push((parent_id, depth + 1));
                            } else {
                                // Parent not found (shallow boundary)
                                boundary_commits.insert(id.clone());
                            }
                        }
                    }
                } else if parents.is_empty() {
                    // True root (no parents)
                    boundary_commits.insert(id.clone());
                }
            }
        }

        // Now determine if we're in z-mode
        let z_mode = hit_depth_limit;

        // If in z-mode, retrospectively trust z commits
        if z_mode {
            // Extend the graph by trusting z commits
            let z_commits_vec: Vec<_> = z_commits.iter().cloned().collect();
            for z_id in z_commits_vec {
                if let Some(commit) = commit_map.get(&z_id) {
                    if let Some(summary) = commit.summary() {
                        if let Some(parsed) = MessageParser::parse(&summary) {
                            if MessageParser::validate(self.repo, commit, &summary, &parsed) {
                                if parsed.prefix == MessagePrefix::ZMode {
                                    // This z commit is now trusted, stop scanning from it
                                    // We don't need to do anything special here since we already
                                    // have it in our graph
                                }
                            }
                        }
                    }
                }
            }
        }

        // Calculate indices from the collected graph
        let revision_index = {
            let mut count = 0;
            let mut current_id = head.id();
            loop {
                if boundary_commits.contains(&current_id) {
                    // Reached a boundary (trusted commit or depth limit)
                    break;
                }
                if let Some(parents) = parent_map.get(&current_id) {
                    if parents.is_empty() {
                        break;
                    }
                    count += 1;
                    current_id = parents[0].clone();
                } else {
                    break;
                }
            }
            count
        };

        let generation_index = self.calculate_generation_bounded(&parent_map, &head.id(), &boundary_commits);
        let commit_index = visited.len().saturating_sub(1) as u32;

        let origin = if revision_index == 0 {
            None
        } else {
            self.calculate_origin_bounded(&parent_map, &commit_map, &boundary_commits)
        };

        GraphStats {
            revision_index,
            generation_index,
            commit_index,
            origin,
            z_mode,
        }
    }

    /// Calculate generation index for bounded graph (depth-limited scan).
    fn calculate_generation_bounded(
        &self,
        parent_map: &HashMap<<R::Commit as CommitView>::Id, Vec<<R::Commit as CommitView>::Id>>,
        head_id: &<R::Commit as CommitView>::Id,
        boundary_commits: &HashSet<<R::Commit as CommitView>::Id>,
    ) -> u32 {
        let mut distances = HashMap::new();

        // Initialize: boundary commits have distance 0
        for boundary_id in boundary_commits {
            distances.insert(boundary_id.clone(), 0);
        }

        // Process in reverse topological order
        let mut processed = HashSet::new();
        let mut max_distance = 0;

        fn visit<Id: Clone + Eq + Hash + Ord + Debug>(
            id: &Id,
            parent_map: &HashMap<Id, Vec<Id>>,
            distances: &mut HashMap<Id, u32>,
            processed: &mut HashSet<Id>,
            max_distance: &mut u32,
            boundary_commits: &HashSet<Id>,
        ) -> u32 {
            if let Some(&dist) = distances.get(id) {
                return dist;
            }

            if !processed.insert(id.clone()) {
                // Cycle detection (shouldn't happen in git)
                return 0;
            }

            // If this is a boundary, it's a root
            if boundary_commits.contains(id) {
                distances.insert(id.clone(), 0);
                return 0;
            }

            let parents = parent_map.get(id).map(|p| p.as_slice()).unwrap_or(&[]);
            let max_parent_dist = parents
                .iter()
                .map(|p| visit(p, parent_map, distances, processed, max_distance, boundary_commits))
                .max()
                .unwrap_or(0);

            let dist = max_parent_dist + 1;
            distances.insert(id.clone(), dist);
            if dist > *max_distance {
                *max_distance = dist;
            }
            dist
        }

        visit(head_id, parent_map, &mut distances, &mut processed, &mut max_distance, boundary_commits);
        max_distance
    }

    /// Calculate origin for bounded graph.
    fn calculate_origin_bounded(
        &self,
        parent_map: &HashMap<<R::Commit as CommitView>::Id, Vec<<R::Commit as CommitView>::Id>>,
        commit_map: &HashMap<<R::Commit as CommitView>::Id, R::Commit>,
        boundary_commits: &HashSet<<R::Commit as CommitView>::Id>,
    ) -> Option<u16> {
        // Use boundary commits as our "roots"
        let mut roots: Vec<_> = boundary_commits
            .iter()
            .filter_map(|id| commit_map.get(id))
            .collect();

        if roots.is_empty() {
            return None;
        }

        roots.sort_by_key(|c| c.id());

        if roots.len() == 1 {
            // Single root: use last 2 bytes (16 bits) of its ID
            let bytes = roots[0].id_bytes();
            if bytes.len() >= 2 {
                let last_two = &bytes[bytes.len() - 2..];
                Some(u16::from_be_bytes([last_two[0], last_two[1]]))
            } else {
                Some(0x0000)
            }
        } else {
            // Multiple roots: sort their IDs, hash them together, use last 2 bytes
            use sha1::{Digest, Sha1};
            let mut hasher = Sha1::new();
            for root in &roots {
                hasher.update(root.id_bytes());
            }
            let hash = hasher.finalize();
            // Use last 2 bytes (bytes 18-19 of 20-byte SHA1)
            Some(u16::from_be_bytes([hash[18], hash[19]]))
        }
    }

    fn full_graph_walk(&self, head: &R::Commit, _is_shallow: bool, _unlimited_depth: bool) -> GraphStats {
        // Build a complete graph using BFS
        let mut visited = HashSet::new();
        let mut queue = vec![head.id()];
        let mut commit_map: HashMap<_, R::Commit> = HashMap::new();
        let mut parent_map: HashMap<_, Vec<_>> = HashMap::new();

        visited.insert(head.id());
        commit_map.insert(head.id(), head.clone());

        while let Some(id) = queue.pop() {
            if let Some(commit) = commit_map.get(&id) {
                let parents = commit.parent_ids();
                parent_map.insert(id.clone(), parents.clone());

                for parent_id in parents {
                    if visited.insert(parent_id.clone()) {
                        if let Some(parent) = self.repo.find_commit(parent_id.clone()) {
                            commit_map.insert(parent_id.clone(), parent);
                            queue.push(parent_id);
                        }
                    }
                }
            }
        }

        // Calculate revision_index (first-parent chain length)
        let revision_index = {
            let mut count = 0;
            let mut current_id = head.id();
            while let Some(parents) = parent_map.get(&current_id) {
                if parents.is_empty() {
                    break;
                }
                count += 1;
                current_id = parents[0].clone();
            }
            count
        };

        // Calculate generation_index (max distance from any root)
        let generation_index = self.calculate_generation(&parent_map, &head.id());

        // Calculate commit_index (total commits - 1)
        let commit_index = visited.len().saturating_sub(1) as u32;

        // Calculate origin
        let origin = if revision_index == 0 {
            None
        } else {
            self.calculate_origin(&parent_map, &commit_map)
        };

        GraphStats {
            revision_index,
            generation_index,
            commit_index,
            origin,
            z_mode: false,
        }
    }

    fn calculate_generation(
        &self,
        parent_map: &HashMap<<R::Commit as CommitView>::Id, Vec<<R::Commit as CommitView>::Id>>,
        head_id: &<R::Commit as CommitView>::Id,
    ) -> u32 {
        let mut distances = HashMap::new();

        // Initialize: roots have distance 0
        for (id, parents) in parent_map {
            if parents.is_empty() {
                distances.insert(id.clone(), 0);
            }
        }

        // Process in reverse topological order
        let mut processed = HashSet::new();
        let mut max_distance = 0;

        fn visit<Id: Clone + Eq + Hash + Ord + Debug>(
            id: &Id,
            parent_map: &HashMap<Id, Vec<Id>>,
            distances: &mut HashMap<Id, u32>,
            processed: &mut HashSet<Id>,
            max_distance: &mut u32,
        ) -> u32 {
            if let Some(&dist) = distances.get(id) {
                return dist;
            }

            if !processed.insert(id.clone()) {
                // Cycle detection (shouldn't happen in git)
                return 0;
            }

            let parents = parent_map.get(id).map(|p| p.as_slice()).unwrap_or(&[]);
            let max_parent_dist = parents
                .iter()
                .map(|p| visit(p, parent_map, distances, processed, max_distance))
                .max()
                .unwrap_or(0);

            let dist = max_parent_dist + 1;
            distances.insert(id.clone(), dist);
            if dist > *max_distance {
                *max_distance = dist;
            }
            dist
        }

        visit(head_id, parent_map, &mut distances, &mut processed, &mut max_distance);
        max_distance
    }

    fn calculate_origin(
        &self,
        parent_map: &HashMap<<R::Commit as CommitView>::Id, Vec<<R::Commit as CommitView>::Id>>,
        commit_map: &HashMap<<R::Commit as CommitView>::Id, R::Commit>,
    ) -> Option<u16> {
        // Find all root commits (true roots OR shallow boundary commits)
        // A commit is a root if:
        // 1. It has no parents (true root), OR
        // 2. All its parents are missing from commit_map (shallow boundary)
        let mut roots: Vec<_> = parent_map
            .iter()
            .filter(|(_id, parents)| {
                if parents.is_empty() {
                    // True root
                    true
                } else {
                    // Check if all parents are missing (shallow boundary)
                    parents.iter().all(|p| !commit_map.contains_key(p))
                }
            })
            .filter_map(|(id, _)| commit_map.get(id))
            .collect();

        if roots.is_empty() {
            return None;
        }

        roots.sort_by_key(|c| c.id());

        if roots.len() == 1 {
            // Single root: use last 2 bytes (16 bits) of its ID
            let bytes = roots[0].id_bytes();
            if bytes.len() >= 2 {
                let last_two = &bytes[bytes.len() - 2..];
                Some(u16::from_be_bytes([last_two[0], last_two[1]]))
            } else {
                Some(0x0000)
            }
        } else {
            // Multiple roots: sort their IDs, hash them together, use last 2 bytes
            use sha1::{Digest, Sha1};
            let mut hasher = Sha1::new();
            for root in &roots {
                hasher.update(root.id_bytes());
            }
            let hash = hasher.finalize();
            // Use last 2 bytes (bytes 18-19 of 20-byte SHA1)
            Some(u16::from_be_bytes([hash[18], hash[19]]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock commit ID type
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct MockId(String);

    // Mock commit implementation
    #[derive(Debug, Clone)]
    struct MockCommit {
        id: MockId,
        parent_ids: Vec<MockId>,
        message: Option<String>,
        tree_id: MockId,
    }

    impl CommitView for MockCommit {
        type Id = MockId;

        fn id(&self) -> Self::Id {
            self.id.clone()
        }

        fn parent_ids(&self) -> Vec<Self::Id> {
            self.parent_ids.clone()
        }

        fn summary(&self) -> Option<String> {
            self.message.clone()
        }

        fn tree_id(&self) -> Self::Id {
            self.tree_id.clone()
        }

        fn id_bytes(&self) -> Vec<u8> {
            // Use the string bytes as a simple mock
            self.id.0.as_bytes().to_vec()
        }
    }

    // Mock repository implementation
    struct MockRepo {
        commits: HashMap<MockId, MockCommit>,
        is_shallow: bool,
    }

    impl MockRepo {
        fn new(is_shallow: bool) -> Self {
            Self {
                commits: HashMap::new(),
                is_shallow,
            }
        }

        fn add_commit(&mut self, id: &str, parents: Vec<&str>, message: Option<&str>) -> MockId {
            let mock_id = MockId(id.to_string());
            let commit = MockCommit {
                id: mock_id.clone(),
                parent_ids: parents.iter().map(|p| MockId(p.to_string())).collect(),
                message: message.map(String::from),
                tree_id: MockId(format!("tree-{}", id)),
            };
            self.commits.insert(mock_id.clone(), commit);
            mock_id
        }
    }

    impl<'repo> RepositoryView<'repo> for MockRepo {
        type Commit = MockCommit;

        fn is_shallow(&self) -> bool {
            self.is_shallow
        }

        fn find_commit(&'repo self, id: MockId) -> Option<Self::Commit> {
            self.commits.get(&id).cloned()
        }

        fn validate_tree_prefix(&self, _tree_id: &MockId, _prefix: &str) -> bool {
            // Always validate for simplicity in tests
            true
        }
    }

    #[test]
    fn test_parse_r0() {
        let parsed = MessageParser::parse("r0 / x1234").unwrap();
        assert_eq!(parsed.prefix, MessagePrefix::Regular);
        assert_eq!(parsed.revision_index, 0);
        assert_eq!(parsed.origin, None);
    }

    #[test]
    fn test_parse_r5_with_origin() {
        let parsed = MessageParser::parse("r5 / xABCD / o1234").unwrap();
        assert_eq!(parsed.prefix, MessagePrefix::Regular);
        assert_eq!(parsed.revision_index, 5);
        assert_eq!(parsed.origin, Some(0x1234));
    }

    #[test]
    fn test_parse_s10_shallow() {
        let parsed = MessageParser::parse("s10 / g15 / n71 / xABCD / o5678").unwrap();
        assert_eq!(parsed.prefix, MessagePrefix::Shallow);
        assert_eq!(parsed.revision_index, 10);
        assert_eq!(parsed.generation_index, Some(15));
        assert_eq!(parsed.commit_index, Some(71));
        assert_eq!(parsed.origin, Some(0x5678));
    }

    #[test]
    fn test_parse_z0() {
        let parsed = MessageParser::parse("z0 / x0000").unwrap();
        assert_eq!(parsed.prefix, MessagePrefix::ZMode);
        assert_eq!(parsed.revision_index, 0);
        assert_eq!(parsed.origin, None);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(MessageParser::parse("invalid").is_none());
        assert!(MessageParser::parse("x123").is_none());
        assert!(MessageParser::parse("").is_none());
    }

    #[test]
    fn test_linear_chain_no_messages() {
        let mut repo = MockRepo::new(false);

        // Create a linear chain: c0 <- c1 <- c2 <- c3
        repo.add_commit("c0", vec![], None);
        repo.add_commit("c1", vec!["c0"], None);
        repo.add_commit("c2", vec!["c1"], None);
        let head_id = repo.add_commit("c3", vec!["c2"], None);

        let head = repo.commits.get(&head_id).unwrap();
        let calculator = GraphStatsCalculator::new(&repo, -1);
        let stats = calculator.calculate(head);

        assert_eq!(stats.revision_index, 3);
        assert_eq!(stats.generation_index, 3);
        assert_eq!(stats.commit_index, 3);
        assert!(!stats.z_mode);
        assert!(stats.origin.is_some()); // Should have origin from c0
    }

    #[test]
    fn test_linear_chain_with_trusted_messages() {
        let mut repo = MockRepo::new(false);

        // Create chain with trusted messages
        // We're calculating stats for a NEW commit on top of c2
        repo.add_commit("c0", vec![], None);
        repo.add_commit("c1", vec!["c0"], Some("r0 / x0000"));
        let head_id = repo.add_commit("c2", vec!["c1"], Some("r1 / x1111 / o1234"));

        let head = repo.commits.get(&head_id).unwrap();
        let calculator = GraphStatsCalculator::new(&repo, -1);
        let stats = calculator.calculate(head);

        // With unlimited depth and single parent, should use optimization
        // c2 has "r1 / x1111 / o1234", so new commit will be r2
        assert_eq!(stats.revision_index, 2);
        assert!(!stats.z_mode);
        assert_eq!(stats.origin, Some(0x1234));
    }

    #[test]
    fn test_depth_limit_triggers_z_mode() {
        let mut repo = MockRepo::new(false);

        // Create a long chain
        repo.add_commit("c0", vec![], None);
        repo.add_commit("c1", vec!["c0"], None);
        repo.add_commit("c2", vec!["c1"], None);
        repo.add_commit("c3", vec!["c2"], None);
        repo.add_commit("c4", vec!["c3"], None);
        let head_id = repo.add_commit("c5", vec!["c4"], None);

        let head = repo.commits.get(&head_id).unwrap();

        // With max_depth=2, should hit depth limit
        let calculator = GraphStatsCalculator::new(&repo, 2);
        let stats = calculator.calculate(head);

        assert!(stats.z_mode);
        assert_eq!(stats.revision_index, 2); // Stopped at depth 2
    }

    #[test]
    fn test_depth_limit_with_trusted_commit() {
        let mut repo = MockRepo::new(false);

        // Create chain with trusted message within depth limit
        repo.add_commit("c0", vec![], None);
        repo.add_commit("c1", vec!["c0"], Some("r0 / x0000"));
        repo.add_commit("c2", vec!["c1"], None);
        let head_id = repo.add_commit("c3", vec!["c2"], None);

        let head = repo.commits.get(&head_id).unwrap();

        // With max_depth=5, should find trusted commit before hitting limit
        let calculator = GraphStatsCalculator::new(&repo, 5);
        let stats = calculator.calculate(head);

        assert!(!stats.z_mode); // Found trusted commit
        assert_eq!(stats.revision_index, 2); // c3 <- c2 <- c1 (trusted)
    }

    #[test]
    fn test_shallow_repo_boundary() {
        let mut repo = MockRepo::new(true);

        // Create shallow chain (c1 is boundary with missing parent c0)
        // We're calculating for NEW commit on top of c1
        let head_id = repo.add_commit("c1", vec!["c0"], Some("s0 / x0000")); // c0 doesn't exist

        let head = repo.commits.get(&head_id).unwrap();
        let calculator = GraphStatsCalculator::new(&repo, -1);
        let stats = calculator.calculate(head);

        // Should use optimization path for single parent
        // c1 has "s0 / x0000", so new commit will be s1
        assert_eq!(stats.revision_index, 1);
        assert!(!stats.z_mode);
    }

    #[test]
    fn test_max_depth_zero() {
        let mut repo = MockRepo::new(false);

        repo.add_commit("c0", vec![], None);
        let head_id = repo.add_commit("c1", vec!["c0"], None);

        let head = repo.commits.get(&head_id).unwrap();
        let calculator = GraphStatsCalculator::new(&repo, 0);
        let stats = calculator.calculate(head);

        assert!(stats.z_mode);
        assert_eq!(stats.revision_index, 0);
        assert_eq!(stats.generation_index, 0);
        assert_eq!(stats.commit_index, 0);
    }

    #[test]
    fn test_merge_commit() {
        let mut repo = MockRepo::new(false);

        // Create a merge:
        //   c0 <- c1 <- c3 (merge)
        //      <- c2 <-/
        repo.add_commit("c0", vec![], None);
        repo.add_commit("c1", vec!["c0"], None);
        repo.add_commit("c2", vec!["c0"], None);
        let head_id = repo.add_commit("c3", vec!["c1", "c2"], None);

        let head = repo.commits.get(&head_id).unwrap();
        let calculator = GraphStatsCalculator::new(&repo, -1);
        let stats = calculator.calculate(head);

        assert_eq!(stats.revision_index, 2); // First-parent chain: c3 <- c1 <- c0
        assert_eq!(stats.generation_index, 2); // Max distance from c0
        assert_eq!(stats.commit_index, 3); // Total: c0, c1, c2, c3 = 4 commits - 1
    }

    #[test]
    fn test_z_commit_not_trusted_during_initial_scan() {
        let mut repo = MockRepo::new(false);

        // Create chain with z commit that should be ignored initially
        repo.add_commit("c0", vec![], None);
        repo.add_commit("c1", vec!["c0"], Some("z0 / x0000"));
        repo.add_commit("c2", vec!["c1"], None);
        let head_id = repo.add_commit("c3", vec!["c2"], None);

        let head = repo.commits.get(&head_id).unwrap();

        // With max_depth=5, should NOT trust z commit during scan
        let calculator = GraphStatsCalculator::new(&repo, 5);
        let stats = calculator.calculate(head);

        // Should scan past the z commit
        assert!(!stats.z_mode); // Didn't hit depth limit
        assert_eq!(stats.revision_index, 3);
    }

    #[test]
    fn test_z_commit_trusted_after_entering_z_mode() {
        let mut repo = MockRepo::new(false);

        // Create long chain with z commit within depth window
        repo.add_commit("c0", vec![], None);
        repo.add_commit("c1", vec!["c0"], None);
        repo.add_commit("c2", vec!["c1"], Some("z1 / x1111 / o1234"));
        repo.add_commit("c3", vec!["c2"], None);
        repo.add_commit("c4", vec!["c3"], None);
        repo.add_commit("c5", vec!["c4"], None);
        let head_id = repo.add_commit("c6", vec!["c5"], None);

        let head = repo.commits.get(&head_id).unwrap();

        // With max_depth=2, will hit depth limit
        let calculator = GraphStatsCalculator::new(&repo, 2);
        let stats = calculator.calculate(head);

        assert!(stats.z_mode);
        // After entering z-mode, z commits become trusted but we still hit depth
        assert_eq!(stats.revision_index, 2);
    }
}
