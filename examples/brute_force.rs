use eyre::Result;

fn main() -> Result<()> {
    // A raw Git commit, such as from `git cat-file commit HEAD`.
    let commit = [
        "tree f39e8a9993875251359f11f3deb43241f7c3ce61\n",
        "parent 2148874f7bbe0aa96041b885e6622af1f9651381\n",
        "author Dev <dev@example.com> 1641915519 +0000\n",
        "committer Dev <dev@example.com> 1641915519 +0000\n",
        "\n",
        "fix(dog): no more puppies\n",
        "\n",
        "Co-Authored-By: Vet <vet@example.com>\n",
    ]
    .join("");
    // Look for a commit ID starting with 0xF00D, or the best prefix match possible.
    let target_hash = &[0xF0, 0x0D];
    // Search through 4 minutes of possible timestamps.
    let min_timestamp = 1_600_000_000;
    let max_timestamp = min_timestamp + 60 * 4;
    let (author_timestamp, commit_timestamp) =
        save::brute_force_timestamps(&commit, target_hash, min_timestamp, max_timestamp);
    println!("To make your commit more F00Dy, try this:");
    println!(
        "GIT_AUTHOR_DATE='{} +0000' GIT_COMMITTER_DATE='{} +0000' git commit --amend --no-edit",
        author_timestamp, commit_timestamp
    );

    Ok(())
}
