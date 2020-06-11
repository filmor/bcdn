use crate::cache::{Cache, CacheResult};
use crate::config::Config;
use actix_web::{http, web, App, Either, HttpResponse, HttpServer, Responder};
use clap;

#[actix_rt::main]
pub async fn run(config: Config, _matches: Option<&clap::ArgMatches<'_>>) -> std::io::Result<()> {
    let mut cache_scope = web::scope("/c/v1");

    HttpServer::new(move || {
        let config = config.clone();
        App::new().service(web::scope("/c/v1").configure(|cfg| configure(&config, cfg)))
        // .service(cache_scope)
    })
    .bind("127.0.0.1:1337")?
    .run()
    .await
}

fn configure(config: &Config, cfg: &mut web::ServiceConfig) {
    for (name, entry) in &config.entries {
        let cache = Cache::new(name, &config);
        let own_scope = web::scope(name)
            .data(cache)
            .route("/f/{filename}", web::get().to(data));

        cfg.service(own_scope);
    }
}

async fn data(path: web::Path<String>, cache: web::Data<Cache>) -> impl Responder {
    match cache.as_ref().get(path.as_ref()).await {
        CacheResult::Ok(manifest) => Either::A(manifest.serve()),
        CacheResult::NotCached { redirect, in_work } => {
            if !in_work {}

            Either::B(
                HttpResponse::TemporaryRedirect()
                    .header(http::header::LOCATION, redirect.to_string())
                    .body(format!("In work: {}", in_work)),
            )
        }
        _ => Either::B(HttpResponse::NotFound().body("Not found")),
    }
}
