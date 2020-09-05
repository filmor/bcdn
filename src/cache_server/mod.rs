use crate::config::Config;
use actix_web::{http, web, App, Either, HttpResponse, HttpServer, Responder};

mod cache;
mod download;
use cache::{Cache, CacheResult};

#[actix_rt::main]
pub async fn run(config: Config, _matches: &clap::ArgMatches<'_>) -> std::io::Result<()> {
    let bind = config.cache.bind.clone();

    log::info!("Starting cache node at {}...", bind);

    HttpServer::new(move || {
        let config = config.clone();
        App::new().service(web::scope("/c/v1").configure(|cfg| configure(&config, cfg)))
        // .service(cache_scope)
    })
    .bind(bind)?
    .run()
    .await
}

fn configure(config: &Config, cfg: &mut web::ServiceConfig) {
    for (name, _entry) in &config.entries {
        let cache = Cache::new(name, &config);
        let own_scope = web::scope(name)
            .data(cache)
            .route("/f/{filename}", web::get().to(data));

        cfg.service(own_scope);
    }
}

async fn data(path: web::Path<String>, cache: web::Data<Cache>) -> impl Responder {
    match cache.as_ref().get(path.as_ref()).await {
        // CacheResult::Ok(digest) => Either::A(digest.serve()),
        CacheResult::NotCached { redirect, in_work } => {
            if !in_work {}

            Either::A(
                HttpResponse::TemporaryRedirect()
                    .header(http::header::LOCATION, redirect.to_string())
                    .body(format!("In work: {}", in_work)),
            )
        }
        _ => Either::B(HttpResponse::NotFound().body("Not found")),
    }
}
