mod cache;
mod config;
mod download;
mod hash_serde;
mod manifest;
mod server;

use crate::config::Config;
use clap::{App, Arg, SubCommand};
use std::fs;
use std::path::Path;

fn main() -> Result<(), std::io::Error> {
    pretty_env_logger::init();
    let m = App::new("bcdn")
        .subcommand(SubCommand::with_name("run"))
        .subcommand(
            SubCommand::with_name("install")
                .arg(Arg::with_name("nginx"))
                .arg(Arg::with_name("systemd")),
        )
        .subcommand(SubCommand::with_name("cleanup"))
        .arg(Arg::with_name("config").default_value("bcdn.toml"))
        .about("Manage or run bcdn")
        .get_matches();

    let cfg_path = m.value_of("config").unwrap();
    let config = fs::read_to_string(Path::new(cfg_path))?;
    let config: Config = toml::from_str(&config)?;

    match m.subcommand() {
        ("run", sub_app) => {
            server::run(config, sub_app);
        }
        _ => (),
    }

    Ok(())
}
