use {crate::cli::Save, eyre::Report, itertools::Itertools, std::path::PathBuf};

/// `save`
pub fn all() -> Result<(), ::eyre::Report> {
    Save::with(|o| o.all = true).save()
}

pub fn paths(paths: impl IntoIterator<Item = impl Into<PathBuf>>) -> Result<(), Report> {
    let _paths = paths.into_iter().map(Into::into).collect_vec();

    // TODO: Implement selective path committing
    // For now, this falls back to committing all changes
    // The original intent was to support: save::paths(&["file1.rs", "file2.rs"])
    // which would only commit those specific files.
    // This requires adding a Vec<PathBuf> field to Save struct and implementing
    // selective git add logic.
    Save::with(|o| o.all = true).save()
}

pub fn with<F: FnOnce(&mut Save) -> T, T>(f: F) -> Result<(), Report> {
    Save::with(f).save()
}
