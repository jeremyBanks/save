use {eyre::Result, git2::Repository};

fn main() -> Result<()> {
    let repo = Repository::open_from_env()?;
    let head = repo.head()?.peel_to_commit()?;
    let generation_number = save::generation_number(&head);
    println!(
        "The generation number of HEAD ({}) in the current repository is: {}",
        &head.id().to_string()[..4],
        generation_number
    );
    Ok(())
}
