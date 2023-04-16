use crate::config::Config;
mod cache_info;

use std::net::ToSocketAddrs;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Router, Server};
use cache_info::CacheInfo;
use hyper::Error;

#[tokio::main]
pub async fn run(config: Config, _matches: &clap::ArgMatches) -> Result<(), Error> {
    let bind = config.proxy.bind.to_socket_addrs().unwrap().next().unwrap();

    log::info!("Starting CDN proxy at {:?}...", bind);

    let mut app = Router::new();
    for name in config.entries.keys() {
        let cache_info = CacheInfo::new(name, &config);
        let sub_router = Router::new()
            .route("/f/:filename", get(data))
            .with_state(Arc::new(cache_info));

        app = app.nest(&format!("/c/{}", name), sub_router);
    }

    Server::bind(&bind).serve(app.into_make_service()).await
}

async fn data(Path(path): Path<String>, State(cache_info): State<Arc<CacheInfo>>) -> Response {
    if let Some(redirect) = cache_info.get_redirect(&path) {
        Redirect::temporary(&redirect.to_string()).into_response()
    } else {
        (StatusCode::NOT_FOUND, "Not found").into_response()
    }
}
