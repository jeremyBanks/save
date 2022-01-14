use {eyre::Result, git2::Repository, save::git2::CommitExt};

fn main() -> Result<()> {
    let repo = Repository::open_from_env()?;
    let head = repo.head()?.peel_to_commit()?;
    let generation_number = head.generation_number();
    println!(
        "The generation number of HEAD ({}) in the current repository is: {}",
        &head.id().to_string()[..4],
        generation_number
    );
    Ok(())
}
