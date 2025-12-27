# Shallow Clone Support

## Overview

The `save` tool works correctly with shallow Git clones, but has limited graph information available.

## Detection

Git stores shallow clone metadata in `.git/shallow` file, which contains commit OIDs of "grafted" commits (commits whose parents are hidden by the shallow boundary).

The `git2` crate provides `Repository::is_shallow()` to detect shallow repositories.

## Behavior in Shallow Clones

When running `save` in a shallow clone:

1. **Graph Statistics Are Limited**: The `graph_stats()` function walks all reachable commits from HEAD, stopping at the shallow boundary where parents aren't available.

2. **Example - Depth 1 Clone**:
   ```
   $ git clone --depth 1 https://github.com/torvalds/linux.git
   $ save --all --no-head

   Commit message: s????-0 / x0430
   ```
   - revision_index = 0 (no first-parent chain beyond shallow boundary)
   - generation_index = 0 (same as revision, omitted from message)
   - commit_index = 0 (1 node in graph, minus 1)
   - Shallow boundary: 1 commit
   - Total reachable: 1 commit

3. **Example - Depth 10 Clone**:
   ```
   $ git clone --depth 10 https://github.com/torvalds/linux.git
   $ save --all --no-head

   Commit message: s????-9 / g15 / n71 / x0430
   ```
   - revision_index = 9 (first-parent chain length)
   - generation_index = 15 (different due to merge history!)
   - commit_index = 71 (72 nodes in graph, minus 1)
   - Shallow boundary: 16 commits (more than depth due to merges)
   - Total reachable: 72 commits

4. **Example - Depth 5 Clone (this repo)**:
   ```
   $ git clone --depth 5 file:///home/user/save test-shallow
   $ cd test-shallow && save --all --no-head

   Commit message: s8B8E-4 / x013D
   ```
   - 's' prefix indicates shallow repository
   - Root hash: 8B8E (always shown for shallow clones)
   - revision_index = 4 (5th commit, 0-indexed)
   - Shallow boundary: 1 commit
   - Total reachable: 5 commits

5. **Example - Full Repository**:
   ```
   $ save --all --no-head

   Commit message: r167 / n169 / x234D
   ```
   - 'r' prefix indicates regular (non-shallow) repository
   - No root hash shown (not needed for regular commits)
   - revision_index = 167 (first-parent chain length)
   - generation_index = 167 (same as revision, omitted)
   - commit_index = 169 (170 nodes in graph, minus 1)

## Measuring "How Shallow"

There are multiple ways to conceptualize shallow depth:

1. **Shallow Boundary Size**: Number of entries in `.git/shallow` file
   - Depth 1 clone typically has 1 entry (the grafted HEAD commit)
   - More complex histories may have multiple boundary commits

2. **Reachable Commits**: Count of commits in the truncated graph
   - This is what `graph_stats.commit_index + 1` represents
   - More reliable than boundary size for understanding available history

3. **Git's --depth Value**: Not stored anywhere after clone completes
   - Cannot be recovered from repository metadata
   - Only the shallow boundary commits are recorded

## Implementation Notes

From `src/git2.rs:270-338`, the `graph_stats()` function:
- Walks ALL parents recursively from HEAD
- Stops when it encounters commits with no parents (either root commits or shallow boundary)
- Uses petgraph to build a directed graph of reachable commits
- Calculates three indices based on this limited graph

The tool does not crash or error in shallow clones - it simply works with the limited history available.

## Finding the Oldest Commits

**Yes, we can determine what the oldest commits are in a shallow repository:**

1. **Read `.git/shallow` file directly**:
   ```bash
   $ cat .git/shallow
   0edc78b82bea85e1b2165d8e870a5c3535919695
   3680fc138e31d8a9e8e344d72c6692e921dbb4a3
   ...
   ```
   These are the commit IDs at the shallow boundary.

2. **Use `Repository::is_shallow()` to detect shallow clones**:
   ```rust
   if repo.is_shallow() {
       // Read .git/shallow to get boundary commits
   }
   ```

3. **Understand the boundary semantics**:
   - Commits in `.git/shallow` are **not** root commits (they have `parent_count() > 0`)
   - Their parent commit objects simply **don't exist** in the repository
   - Calling `commit.parent(i)` on these commits returns `Err("object not found")`
   - This is why they appear as "leaf nodes" in graph traversal

**Example from depth=10 Linux clone:**
```
Boundary commit: 0edc78b82bea - "x86/msi: Make irq_retrigger() functional..."
  parent_count(): 1
  parent(0): ERROR - object not found (21433d3e3c...)
```

The commit has a parent reference encoded in its object, but that parent doesn't exist in `.git/objects/`.

## Recommendation

For shallow clones, the commit_index provides the most useful information: the number of commits reachable from HEAD minus 1. This gives users a sense of how much history is available in their shallow clone.

To find the "oldest" commits (shallow boundary), read `.git/shallow` directly or detect when `commit.parent(i)` fails with "object not found".

No code changes are needed - the tool handles shallow clones correctly as-is.
