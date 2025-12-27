# Implementation Status

## Completed ✅

### Core Infrastructure
- **graph_stats module**: Abstract interface for graph statistics calculation
  - `CommitView` trait: Abstract commit interface
  - `RepositoryView` trait: Abstract repository interface
  - `MessageParser`: Parse commit messages in our format
  - `GraphStatsCalculator`: Main calculation engine

### git2 Integration
- `CommitView` implementation for `git2::Commit`
- `RepositoryView` implementation for `git2::Repository`
- Proper lifetime handling for git2 types

### Origin Tracking
- Origin calculation from root commit IDs
- Single root: last 2 bytes of root OID
- Multiple roots: last 2 bytes of SHA1(sorted root OIDs)
- Shallow boundary detection: treats missing parents as roots
- Origin field in commit messages: `/oHHHH`

### CLI Integration
- GraphStatsCalculator integrated into save CLI
- `--max-depth` parameter (default: 255)
- Prefix determination: r/s/z based on mode
- Message formatting with all fields

### Testing & Dogfooding
- Tool successfully commits using itself
- Example messages: `s183 / g184 / n185 / x6423 / o077C`
- All fields working correctly

## In Progress ⏳

### Z-Mode Depth-Limited Scanning
- **Current state**: Skeleton exists but not implemented
- **What's needed**:
  1. Per-path scanning up to max_depth
  2. Trust hierarchy: r (always if not shallow), s (if shallow), z (only in z-mode)
  3. Collect z commits during scan without trusting
  4. Enter z-mode if ANY path hits depth limit
  5. Retrospectively trust collected z commits
  6. Declare z0 for paths that hit depth with no trusted commit
  7. Calculate indices based on limited graph

### Current Behavior
- `full_graph_walk` ignores max_depth parameter
- Always does full graph traversal
- Never enters z-mode
- Works correctly but no depth limiting

## Not Started 🔴

### Unit Tests for graph_stats
- Mock commit/repository implementations
- Test cases for z-mode scenarios
- Test depth limiting behavior
- Test trust hierarchy
- Test multiple paths with different outcomes

### Advanced Features
- Rollback handling (deferred to FUTURE_TOPICS.md)
- History normalization (deferred to FUTURE_TOPICS.md)
- `--rebuild` flag integration

## Next Steps

1. Implement `depth_limited_scan` function in GraphStatsCalculator
2. Replace `full_graph_walk` call with conditional:
   - Use `full_graph_walk` if max_depth < 0 (unlimited)
   - Use `depth_limited_scan` if max_depth >= 0
3. Add unit tests with mock implementations
4. Test on large repositories to verify bounded complexity
5. Document z-mode behavior and examples

## Testing Notes

- Repository is shallow (has `.git/shallow`)
- Current messages show 's' prefix correctly
- Origin field working: `o077C` represents shallow boundary
- Indices incrementing correctly
- Tree hash brute-forcing working
