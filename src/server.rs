use crate::cache::Cache;
use crate::config::Config;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::runtime::Runtime;
use warp;
use warp::Filter;

pub fn run(
    config: Config,
    _matches: Option<&clap::ArgMatches>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut rt = Runtime::new()?;
    rt.block_on(run_async(config))
}

async fn run_async(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let mut caches: HashMap<String, _> = HashMap::new();

    for name in config.entries.keys() {
        let cache = Cache::new(name, &config);
        &caches.insert(name.clone(), cache);
    }

    log::info!("Config: {:?}", config);
    log::info!("Cache keys: {:?}", caches.keys());

    let caches = Arc::new(RwLock::new(caches));

    let data = warp::path!("data" / "v1" / String / String)
        .and(warp::get())
        .and(warp::any().map(move || caches.clone()))
        .and_then(
            move |entry, name, caches: Arc<RwLock<HashMap<String, _>>>| async move {
                log::info!("Request for {}/{}", entry, name);
                if let Some(cache) = caches.read().unwrap().get(&entry) {
                    let s = format!("Entry: {} Name: {}", &entry, &name).to_owned();
                    Ok(s)
                } else {
                    Err(warp::reject::not_found())
                }
            },
        );

    let routes = data;

    warp::serve(routes).run(([0, 0, 0, 0], 1337)).await;

    Ok(())
}
