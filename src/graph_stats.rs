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
}

/// Abstract interface for repository operations needed by the algorithm.
pub trait RepositoryView {
    type Commit: CommitView;

    /// Check if this is a shallow clone.
    fn is_shallow(&self) -> bool;

    /// Find a commit by its ID.
    fn find_commit(&self, id: <Self::Commit as CommitView>::Id) -> Option<Self::Commit>;

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
    pub fn validate<R: RepositoryView>(
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
pub struct GraphStatsCalculator<'a, R: RepositoryView> {
    repo: &'a R,
    max_depth: i32,
}

impl<'a, R: RepositoryView> GraphStatsCalculator<'a, R> {
    pub fn new(repo: &'a R, max_depth: i32) -> Self {
        Self { repo, max_depth }
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
    pub fn calculate(&self, _head: &R::Commit) -> GraphStats {
        // TODO: Implement the full z-mode algorithm
        // For now, return default to allow the code to compile
        GraphStats::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
