# Commit Message Format

## Overview

The `save` tool automatically generates commit messages that encode information about the commit's position in the repository's history and whether the repository is shallow.

## Format

```
[r|s|z]N [/ gG] [/ nC] [/ xHHHH] [/ oHHHH]
```

### Components

1. **Prefix**: `r`, `s`, or `z`
   - `r`: Regular (non-shallow) repository
   - `s`: Shallow repository (cloned with `--depth`)
   - `z`: Z-mode (hit depth limit during scan)

2. **Revision Index**: `N`
   - Number of commits in the first-parent chain from this commit
   - Increments by 1 for each commit along the primary branch
   - `0` for root commits (no parents)

3. **Generation Index** (optional): `/ gG`
   - Maximum topological distance from any root commit
   - Only shown if different from revision index (indicates merge history)

4. **Commit Index** (optional): `/ nC`
   - Total number of reachable commits minus 1
   - Only shown if different from generation index
   - In shallow clones, reflects available history

5. **Tree Hash**: `/ xHHHH`
   - First 4 hex digits of the tree hash (uppercase)
   - Brute-forced by manipulating commit timestamp
   - Omitted if tree is empty

6. **Origin** (optional): `/ oHHHH`
   - Last 4 hex digits (2 bytes) of root commit ID
   - For single root: last 2 bytes of that root's OID
   - For multiple roots: last 2 bytes of SHA1(sorted root OIDs)
   - Omitted for root commits themselves (r0/s0/z0)
   - Used to detect history changes and merges

## Examples

### Normal Repository

```
r0 / xABCD              # First commit (root)
r1234-1 / x5678         # Second commit (shows root hash after r0)
r2 / x9ABC              # Third commit (no root hash needed)
r123 / x DEF1           # 124th commit, linear history
r50 / g75 / n100 / x23  # Commit with merge history
```

### Shallow Repository

```
s8B8E-0 / x1234         # Root in shallow clone (depth=1)
s8B8E-4 / x5678         # 5th commit in shallow history
s8B8E-9 / g15 / n71 / xABCD  # Depth=10 clone with merges
```

The `s` prefix immediately indicates the repository is shallow, and the root hash is always included to track the shallow boundary.

### History Changes

```
r1234-50 / xABCD        # Normal commit
r5678-51 / xDEF0        # After merging unrelated history
```

When the root hash changes (e.g., from merging an unrelated history or grafting), the new root hash is shown to indicate the change in the repository's foundation.

## Root Hash Calculation

The root hash is calculated by:
1. Finding all commits with no parents in the reachable history
2. Sorting their OIDs
3. Concatenating and hashing them with SHA1
4. Taking the first 16 bits (2 bytes) as a `u16`
5. Formatting as 4 uppercase hexadecimal digits

In shallow clones, "root commits" are those at the shallow boundary - they have parent references in their commit objects, but those parent objects don't exist in the repository.

## Purpose

The commit message format serves several purposes:

1. **Version Tracking**: Revision index provides a simple, incrementing version number
2. **Shallow Detection**: The `s` prefix immediately indicates limited history
3. **History Integrity**: Root hash changes alert to history modifications
4. **Merge Detection**: Generation and commit indices reveal complex history
5. **Visual Appeal**: Tree hash prefix provides recognizable commit identifiers

## Implementation

See:
- `src/git2.rs:186-192` - `GraphStats` struct definition
- `src/git2.rs:335-352` - Root hash calculation
- `src/cli.rs:454-510` - Message formatting logic
