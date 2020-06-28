use crate::config::Config;
use actix_web::{http, web, App, Either, HttpResponse, HttpServer, Responder};
mod cache_info;
use cache_info::CacheInfo;

#[actix_rt::main]
pub async fn run(config: Config, _matches: &clap::ArgMatches<'_>) -> std::io::Result<()> {
    let bind = config.proxy.bind.clone();

    log::info!("Starting CDN proxy at {}...", bind);

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
    for name in config.entries.keys() {
        let cache_info = CacheInfo::new(name, &config);
        let own_scope = web::scope(name)
            .data(cache_info)
            .route("/f/{filename}", web::get().to(data));

        cfg.service(own_scope);
    }
}

async fn data(path: web::Path<String>, cache_info: web::Data<CacheInfo>) -> impl Responder {
    let cache_info = cache_info.as_ref();

    if let Some(redirect) = cache_info.get_redirect(path.as_ref()) {
        Either::A(
            HttpResponse::TemporaryRedirect()
                .header(http::header::LOCATION, redirect.to_string())
                .body("Redirect"),
        )
    } else {
        Either::B(HttpResponse::NotFound().body("Not found"))
    }
}
