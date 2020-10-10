use std::collections::HashMap;

use crate::config::Config;

mod cache_info;
use cache_info::CacheInfo;
use rocket::{State, config::ConfigBuilder, config::Environment, response::Redirect, response::Responder};

pub fn run(config: Config, _matches: &clap::ArgMatches<'_>) -> std::io::Result<()> {
    log::info!("Starting CDN proxy");

    let cache_infos: HashMap<&str, _> = config
        .entries
        .keys()
        .map(|n| (n.as_str(), CacheInfo::new(n, &config)))
        .collect();

    let rkt_config = ConfigBuilder::new(Environment::Staging)
        .address(config.proxy.address)
        .port(config.proxy.port)
        .finalize()?;

    rocket::custom(rkt_config)
        .mount("/c/v1", rocket::routes![data])
        .manage(cache_infos)
        .launch();

    unimplemented!()
}

#[get("/<cache>/f/<filename>")]
fn data(cache: String, filename: String, cache_infos: State<'_, HashMap<&str, CacheInfo>>) -> impl Responder {
    let cache_info = cache_infos.get(cache.as_str())?;
    
    if let Some(redirect) = cache_info.get_redirect(filename.as_str()) {
        return Some(Redirect::temporary(redirect.to_string()))
    }
    None
}
