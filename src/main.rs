fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(save::cli::main(save::cli::init())?)
}
