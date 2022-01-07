//! Internal Git logic.

/// The generation index is the number of edges of the longest path between the
/// given commit and an initial commit (one with no parents, which has an
/// implicit generation index of 0).
pub fn find_generation_index(commit: &git2::Commit) -> u32 {
    let mut generation_index = 0;
    let mut commits = vec![commit.clone()];

    // naive solution: walk the entire graph depth-first.
    // this could be pathological with a lot of branches.
    // we could use a smarter algorithm, and/or if we come
    // across a commit whose message matches the expected
    // format and tree hash, we trust that it's accurate.
    // however, that could be honestly mangled by a rebase
    // or something, so it might not do.
    loop {
        let mut next_generation_commits = vec![];
        for commit in commits.iter() {
            next_generation_commits.extend(commit.parents());
        }

        if next_generation_commits.is_empty() {
            break;
        } else {
            generation_index += 1;
            commits = next_generation_commits;
            continue;
        }
    }

    generation_index
}

// pub fn brute_commit(commit: &git2::Commit, range: u32, target: &[u8]) ->
// (i64, i64) {     todo!()
// }
