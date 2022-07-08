use {crate::cli::Save, eyre::Report, itertools::Itertools, std::path::PathBuf};

/// `save`
pub fn all() -> Result<(), ::eyre::Report> {
    Save::with(|o| o.all = true).save()
}

pub fn paths(paths: impl IntoIterator<Item = impl Into<PathBuf>>) -> Result<(), Report> {
    let paths = paths.into_iter().map(Into::into).collect_vec();

    todo!()
}

pub fn with<F: FnOnce(&mut Save) -> T, T>(f: F) -> Result<(), Report> {
    Save::with(f).save()
}
