use {::clap::Parser, ::eyre::Report, ::save::cli::Save};

fn main() -> Result<(), Report> {
    ::color_eyre::install()?;
    Save::parse().save()
}
