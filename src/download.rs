use crate::manifest::Manifest;
use blake3::Hasher;
use futures_util::StreamExt;
use reqwest::Client;
use std::fs;
use std::io::Write;
use std::path::Path;
use thiserror::Error;
use url::Url;

async fn download<P>(client: &Client, url: Url, path: P) -> Result<Manifest, DownloadError>
where
    P: AsRef<Path>,
{
    let resp = client.get(url).send().await?;
    let headers = resp.headers();
    let content_type: String = if let Some(value) = headers.get(reqwest::header::CONTENT_TYPE) {
        value.to_str().unwrap().to_owned()
    } else {
        "unknown".to_owned()
    };

    let mut hasher = Hasher::new();
    let mut output = fs::File::create(&path)?;

    let mut stream = resp.bytes_stream();

    while let Some(item) = stream.next().await {
        let item = item?;
        hasher.write_all(&item).unwrap();
        output.write_all(&item)?;
    }

    let hash = hasher.finalize();

    let res = Manifest::new(path, &content_type, hash);

    Ok(res)
}

#[derive(Error, Debug)]
enum DownloadError {
    #[error("IO error")]
    IoError(#[from] std::io::Error),

    #[error("HTTP error")]
    RequestError(#[from] reqwest::Error),
}
