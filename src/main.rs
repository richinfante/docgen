use std::io::{self, Read};
extern crate docgen;

fn main() -> io::Result<()> {
    env_logger::init();

    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    println!("{}", docgen::render(&mut buffer, serde_json::from_str(r###"{ "name": "Rich" }"###).unwrap()));
    Ok(())
}
