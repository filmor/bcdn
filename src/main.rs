mod cache;
mod config;
mod download;
mod hash_serde;
mod manager;
mod manifest;
mod server;

use crate::config::Config;
use crate::manager::CacheManager;
use clap::{App, Arg, SubCommand};
use std::fs;
use std::path::Path;
use warp;
use warp::Filter;
use tokio::runtime::Runtime;

fn main() -> Result<(), std::io::Error> {
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
        ("run", sub_app) => { run(config, sub_app); }
        _ => (),
    }

    Ok(())
}

fn run(config: Config, matches: Option<&clap::ArgMatches>) -> Result<(), Box<dyn std::error::Error>> {
    let root_path = Path::new(&config.root_path);
    let man = CacheManager::new();

    let mut rt = Runtime::new()?;
    rt.block_on(async {
        let routes = warp::any().map(|| "Hello world!");
        warp::serve(routes).run(([0, 0, 0, 0], 1337)).await;
    });

    Ok(())
}
