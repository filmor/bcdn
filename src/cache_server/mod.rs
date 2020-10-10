use std::collections::HashMap;

use crate::config::Config;

mod cache;
mod download;
use cache::{Cache, CacheResult};
use rocket::{State, config::ConfigBuilder, config::Environment};

pub async fn run(config: Config, _matches: &clap::ArgMatches<'_>) -> Result<(), rocket::error::Error> {
    log::info!("Starting cache node");
    
    let caches: HashMap<&str, _> = config
        .entries
        .keys()
        .map(|n| (n.as_str(), Cache::new(n, &config)))
        .collect();
    
    let rkt_config = ConfigBuilder::new(Environment::Staging)
        .address(config.cache.address)
        .port(config.cache.port)
        .finalize()?;

    rocket::ignite()
        .mount("/c/v1", rocket::routes![data])
        .manage(caches)
        .launch()
        .await
}

#[get("/<cache>/f/<filename>")]
async fn data(cache: String, filename: String, caches: State<'_, HashMap<&str, Cache>>) -> Option<String> {
    let cache = caches.get(cache.as_str())?;
    cache.get(filename.as_str()).await;
    None
}
