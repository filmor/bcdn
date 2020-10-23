use std::{collections::HashMap, sync::Arc};

use crate::config::Config;
use actix_web::{http, web, App, Either, HttpResponse, HttpServer, Responder};

mod cache;
mod download;
use cache::{Cache, CacheResult};

use self::download::DownloadPool;

#[actix_rt::main]
pub async fn run(config: Config, _matches: &clap::ArgMatches<'_>) -> std::io::Result<()> {
    let bind = config.cache.bind.clone();

    log::info!("Starting cache node at {}...", bind);

    let caches: Arc<HashMap<_, _>> = Arc::new(
        config
            .entries
            .iter()
            .map(|(k, _v)| (k.clone(), web::Data::new(Cache::new(k, &config).unwrap())))
            .collect(),
    );

    HttpServer::new(move || {
        let caches = caches.clone();
        let pool = DownloadPool::new(&config);

        App::new()
            .app_data(web::Data::new(pool))
            .service(web::scope("/c/v1").configure(|cfg| configure(caches.clone(), cfg)))
        // .service(cache_scope)
    })
    .bind(bind)?
    .run()
    .await
}

fn configure(caches: Arc<HashMap<String, web::Data<Cache>>>, cfg: &mut web::ServiceConfig) {
    for (name, cache) in caches.iter() {
        let cache = cache.clone();
        let own_scope = web::scope(&name)
            .app_data(cache)
            .route("/f/{filename}", web::get().to(data));

        cfg.service(own_scope);
    }
}

async fn data(path: web::Path<String>, cache: web::Data<Cache>, pool: web::Data<DownloadPool>) -> impl Responder {
    let cache = cache.as_ref();
    let pool = pool.as_ref();

    match cache.get(path.as_ref()).await {
        // CacheResult::Ok(digest) => Either::A(digest.serve()),
        // CacheResult::Incomplete(digest) => Either::A(digest.serve()),
        CacheResult::NotCached => {
            let redirect = "redirect-location";

            Either::A(
                HttpResponse::TemporaryRedirect()
                    .header(http::header::LOCATION, redirect.to_string())
                    .body(format!("In work")),
            )
        }
        _ => Either::B(HttpResponse::NotFound().body("Not found")),
    }
}
