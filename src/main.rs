fn main() -> Result<(), Box<dyn ::std::error::Error>> {
    let args = save::cli::init();
    Ok(save::cli::main(args)?)
}
