use std::io::{self, Read};
extern crate docgen;
use clap::{App, Arg, SubCommand};
use glob::glob;
mod render;

#[macro_use]
extern crate log;

use std::env;
use std::io::Write;
use log::LevelFilter;
use env_logger::Builder;
use colored::*;

fn main() -> io::Result<()> {
    let mut builder = Builder::from_default_env();

    builder.format(|buf, record| writeln!(buf, "{:>8} {}", match record.level() {
        log::Level::Trace => "Trace".white().dimmed(),
        log::Level::Debug => "Debug".blue(),
        log::Level::Warn => "Warning".yellow(),
        log::Level::Error => "Error".red(),
        log::Level::Info => "Info".green()
    }, record.args()))
           .filter(None, LevelFilter::Info)
           .init();

    let matches = App::new("docgen")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("input")
                .short("i")
                .default_value("./**/{*.html,*.htm,*.md}")
                .help("Input File Glob Expression"),
        )
        .get_matches();

    let pattern = matches.value_of("input").unwrap();
    debug!("got pattern: {}", pattern);
    for entry in glob(pattern).expect("Failed to read input glob pattern") {
        match entry {
            Ok(path) => {
                let res = docgen::render_recursive(&path, std::rc::Rc::new(None), None, None);
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
