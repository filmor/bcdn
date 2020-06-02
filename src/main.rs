mod cache;
mod config;
mod download;
mod hash_serde;
mod manifest;
mod server;

use crate::cache::Cache;
use crate::config::Config;
use clap::{App, Arg, SubCommand};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tokio::runtime::Runtime;
use warp;
use warp::Filter;

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
        ("run", sub_app) => {
            run(config, sub_app);
        }
        _ => (),
    }

    Ok(())
}

fn run(
    config: Config,
    matches: Option<&clap::ArgMatches>,
) -> Result<(), Box<dyn std::error::Error>> {
    let root_path = Path::new(&config.root_path);
    let mut caches = HashMap::new();

    for name in config.entries.keys() {
        let cache = Cache::new(name, &config);
        caches.insert(name, cache);
    }

    let mut rt = Runtime::new()?;
    rt.block_on(async {
        let data = warp::path!("data" / "v1" / String / String)
            .and(warp::get())
            .map(|entry, name| format!("Entry: {} Name: {}", entry, name));

        let routes = data;

        warp::serve(routes).run(([0, 0, 0, 0], 1337)).await;
    });

    Ok(())
}
