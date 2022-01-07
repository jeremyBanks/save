fn main() -> Result<(), Box<dyn ::std::error::Error>> {
    let args = save::init();
    Ok(save::main(args)?)
}
