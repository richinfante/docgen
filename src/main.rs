use std::io::{self, Read};
extern crate docgen;
use clap::{App, Arg, SubCommand};
use glob::glob;
mod render;

#[macro_use]
extern crate log;

fn main() -> io::Result<()> {
    env_logger::init();

    let matches = App::new("My Super Program")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::with_name("input")
                .short("i")
                .default_value("./**/{*.html,*.htm,*.md}")
                .help("print debug information verbosely"),
        )
        .get_matches();

    let pattern = matches.value_of("input").unwrap();
    debug!("got pattern: {}", pattern);
    for entry in glob(pattern).expect("Failed to read input glob pattern") {
        match entry {
            Ok(path) => {
                let res = docgen::render_recursive(&path, std::rc::Rc::new(None), None);
                println!("{}", res);
                break;
            }
            Err(e) => error!("{:?}", e),
        }
    }

    // let mut buffer = String::new();
    // io::stdin().read_to_string(&mut buffer)?;

    // println!(
    //     "{}",
    //     docgen::render(
    //         &mut buffer,
    //         serde_json::from_str(r###"{ "name": "Rich" }"###).unwrap()
    //     )
    // );
    Ok(())
}
