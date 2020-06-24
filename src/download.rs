use crate::manifest::Manifest;
use blake3::Hasher;
use futures_util::StreamExt;
use reqwest::Client;
use std::fs;
use std::io::Write;
use std::path::Path;
use thiserror::Error;
use url::Url;

pub async fn download<P>(client: &Client, url: Url, path: P) -> Result<Manifest, DownloadError>
where
    P: AsRef<Path>,
{
    let path: &Path = path.as_ref();
    let file_name = if let Some(file_name) = path.file_name() {
        file_name.to_string_lossy()
    } else {
        return Err(DownloadError::PathError);
    };

    let resp = client.get(url.clone()).send().await?
        .error_for_status()?;

    let headers = resp.headers();
    let content_type: String = if let Some(value) = headers.get(reqwest::header::CONTENT_TYPE) {
        value.to_str().unwrap().to_owned()
    } else {
        "unknown".to_owned()
    };

    let download_fn = format!(".{}.download", file_name);
    let download_path = path.with_file_name(download_fn);

    log::debug!("Downloading {} to {}", url, download_path.to_string_lossy());

    let mut hasher = Hasher::new();
    fs::create_dir_all(path.parent().unwrap())?;
    let mut output = fs::File::create(&download_path)?;

    let mut stream = resp.bytes_stream();

    while let Some(item) = stream.next().await {
        let item = item?;
        hasher.write_all(&item).unwrap();
        output.write_all(&item)?;

        // TODO Return this as a stream as well s.t. we can pass it through
    }

    let hash = hasher.finalize();

    fs::rename(download_path, path)?;

    let res = Manifest::new(path, &content_type, hash);
    Ok(res)
}

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("IO error")]
    IoError(#[from] std::io::Error),

    #[error("HTTP error")]
    RequestError(#[from] reqwest::Error),

    #[error("Path error")]
    PathError,
}
