fn main() -> ::eyre::Result<()> {
    Ok(::save::cli::main(::save::cli::init())?)
}
