use cache::{Cache, CacheResult};
use std::{collections::HashMap, sync::Arc};

use crate::config::Config;

mod cache;
mod download;
use axum::{
    extract::{FromRef, Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use hyper::Error;
use std::net::ToSocketAddrs;

use self::download::DownloadPool;

#[derive(Clone)]
struct AppState {
    caches: Arc<HashMap<String, Cache>>,
    pool: Arc<DownloadPool>,
}

impl FromRef<AppState> for Arc<DownloadPool> {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.pool.clone()
    }
}

impl FromRef<AppState> for Arc<HashMap<String, Cache>> {
    fn from_ref(input: &AppState) -> Self {
        input.caches.clone()
    }
}

#[tokio::main]
pub async fn run(config: Config, _matches: &clap::ArgMatches) -> Result<(), Error> {
    let bind = config.proxy.bind.to_socket_addrs().unwrap().next().unwrap();

    log::info!("Starting cache node at {:?}...", bind);

    let caches = Arc::new(
        config
            .entries
            .iter()
            .map(|(k, _v)| (k.clone(), Cache::new(k, &config).unwrap()))
            .collect(),
    );
    let pool = Arc::new(DownloadPool::new(&config));

    let app = axum::Router::new()
        .route("/c/:name/f/:filename", get(data))
        .with_state(AppState { caches, pool });

    axum::Server::bind(&bind)
        .serve(app.into_make_service())
        .await
}

async fn data(
    Path(name): Path<String>,
    Path(filename): Path<String>,
    State(pool): State<Arc<DownloadPool>>,
    State(caches): State<Arc<HashMap<String, Cache>>>,
) -> Response {
    let cache = caches.get(&name).unwrap();
    let filename = check_filename(&filename);

    match cache.get(filename).await {
        // CacheResult::Ok(digest) => Either::A(digest.serve()),
        // CacheResult::Incomplete(digest) => Either::A(digest.serve()),
        CacheResult::NotCached => {
            log::info!(
                "File {} is not cached for {}, enqueuing",
                filename,
                cache.name,
            );
            let enq_res = pool.enqueue(cache, filename).await;
            log::info!(
                "File {} is not cached for {}, download state: {:?}",
                filename,
                cache.name,
                enq_res
            );

            if enq_res.percentage() > 30 {
                // PartialNamedFile
            }
            let redirect = cache.get_redirect(filename);
            Redirect::temporary(&redirect.to_string()).into_response()
        }
        CacheResult::Ok(digest) => {
            (
                StatusCode::NOT_IMPLEMENTED,
                digest.get_file_path().to_string_lossy().to_string(),
            )
                .into_response()
            /*
            let f = NamedFile::open(digest.get_file_path())
                .unwrap()
                .set_content_type(digest.content_type.parse().unwrap());
            Either::B(f) */
        }
        _ => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

fn check_filename<'a>(filename: &'a str) -> &'a str {
    if filename.contains('/') {
        panic!("Invalid filename");
    }
    filename
}
