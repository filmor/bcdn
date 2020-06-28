mod cache_server;
mod config;
mod digest;
mod proxy_server;
mod util;

use config::Config;

use clap::{App, Arg, ArgMatches, SubCommand};
use std::fs;
use std::path::Path;

fn main() -> Result<(), std::io::Error> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }

    pretty_env_logger::init();
    let m = App::new("bcdn")
        .subcommand(
            SubCommand::with_name("cache")
                .subcommand(SubCommand::with_name("run"))
                .subcommand(SubCommand::with_name("install"))
                .subcommand(SubCommand::with_name("clean")),
        )
        .subcommand(
            SubCommand::with_name("proxy")
                .subcommand(SubCommand::with_name("run"))
                .subcommand(SubCommand::with_name("install")),
        )
        .arg(
            Arg::with_name("config")
                .long("config")
                .short("c")
                .default_value("bcdn.toml"),
        )
        .about("Manage or run bcdn")
        .get_matches();

    let cfg_path = m.value_of("config").unwrap();
    let config = fs::read_to_string(Path::new(cfg_path))?;
    let config: Config = toml::from_str(&config)?;

    match m.subcommand() {
        ("cache", Some(matches)) => cache(config, matches),
        ("proxy", Some(matches)) => proxy(config, matches),
        _ => {
            println!("{}", m.usage());
            Ok(())
        }
    }
}

fn cache(config: Config, matches: &ArgMatches) -> Result<(), std::io::Error> {
    match matches.subcommand() {
        ("run", _) => cache_server::run(config, matches),
        _ => {
            println!("{}", matches.usage());
            Ok(())
        }
    }
}

fn proxy(config: Config, matches: &ArgMatches) -> Result<(), std::io::Error> {
    match matches.subcommand() {
        ("run", _) => proxy_server::run(config, matches),
        _ => {
            println!("{}", matches.usage());
            Ok(())
        }
    }
}
