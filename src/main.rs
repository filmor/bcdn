mod cache_server;
mod config;
mod proxy_server;
mod util;

use config::Config;

use hyper::Error;
use clap::{Arg, ArgMatches, Command};
use std::fs;
use std::path::Path;

fn main() -> Result<(), Error> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    pretty_env_logger::init();
    let m = Command::new("bcdn")
        .subcommand(
            Command::new("cache")
                .subcommand(Command::new("run"))
                .subcommand(Command::new("install"))
                .subcommand(Command::new("clean")),
        )
        .subcommand(
            Command::new("proxy")
                .subcommand(Command::new("run"))
                .subcommand(Command::new("install")),
        )
        .arg(
            Arg::new("config")
                .long("config")
                // .short("c")
                .default_value("bcdn.toml"),
        )
        .about("Manage or run bcdn")
        .get_matches();

    let cfg_path = m.get_one::<String>("config").unwrap();
    let config = fs::read_to_string(Path::new(cfg_path)).unwrap();
    let config: Config = toml::from_str(&config).unwrap();

    match m.subcommand() {
        Some(("cache", matches)) => cache(config, matches),
        Some(("proxy", matches)) => proxy(config, matches),
        _ => {
            // println!("{}", m.usage());
            Ok(())
        }
    }
}

fn cache(config: Config, matches: &ArgMatches) -> Result<(), Error> {
    match matches.subcommand() {
        Some(("run", _)) => cache_server::run(config, matches),
        _ => {
            // println!("{}", matches.usage());
            Ok(())
        }
    }
}

fn proxy(config: Config, matches: &ArgMatches) -> Result<(), Error> {
    match matches.subcommand() {
        Some(("run", _)) => proxy_server::run(config, matches),
        _ => {
            // println!("{}", matches.usage());
            Ok(())
        }
    }
}
