use std::{fs, io::Write, path::PathBuf};

use blake3::Hasher;
use futures_util::StreamExt;
use reqwest::{Client, Url};
use thiserror::Error;
use tokio::task;

use crate::{
    cache_server::cache::DigestError,
    util::rpc::{rpc, RpcError, RpcHandle, RpcReceiver},
};

use super::Digest;

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MB

pub struct Downloader<K: Clone + Send + 'static> {
    _join_handle: task::JoinHandle<()>,
    rpc: RpcHandle<Command<K>, Reply<K>>,
}

impl<K: Clone + Send + 'static> Downloader<K> {
    pub fn new(client: Client) -> Self {
        let (tx, rx) = rpc();

        let join_handle = tokio::task::spawn(async move {
            let client = client.clone();
            idle_task(client, rx).await;
        });

        Downloader {
            _join_handle: join_handle,
            rpc: tx,
        }
    }

    pub async fn start(&self, key: K, url: Url, path: PathBuf) -> Result<(), ()> {
        match self.rpc.call(Command::Start(key, url, path)).await {
            Ok(Reply::Ok) => Ok(()),
            _ => Err(()),
        }
    }

    pub async fn stop(&self) {
        let _ = self.rpc.call(Command::Stop).await;
    }

    pub async fn status(&self) -> Option<DownloadStatus<K>> {
        self.rpc.call(Command::Status).await.ok().map(|r| match r {
            Reply::Idle => DownloadStatus::Idle,
            Reply::Downloading {
                key,
                downloaded,
                size,
            } => DownloadStatus::Downloading {
                key,
                downloaded,
                size,
            },
            _ => unreachable!(),
        })
    }
}

impl<K: Clone + Send + 'static> Drop for Downloader<K> {
    fn drop(&mut self) {
        let rpc = self.rpc.clone();
        tokio::task::spawn(async move { let _ =rpc.call(Command::Quit).await; });        
    }
}

enum Command<K> {
    Start(K, Url, PathBuf),
    Stop,
    Status,
    Quit,
}

enum Reply<K> {
    Ok,
    Error,
    Downloading {
        key: K,
        downloaded: usize,
        size: usize,
    },
    Idle,
}

pub enum DownloadStatus<K> {
    Idle,
    Downloading {
        key: K,
        downloaded: usize,
        size: usize,
    },
}

async fn idle_task<K: Clone + Send>(client: Client, mut rx: RpcReceiver<Command<K>, Reply<K>>) {
    let mut cont = true;
    let mut current_task = None;
    while cont {
        if let Err(_) = rx
            .reply_once(|q| match q {
                Command::Start(key, url, path) => {
                    current_task = Some((key, url, path));
                    Reply::Ok
                }
                Command::Stop => Reply::Ok,
                Command::Status => Reply::Idle,
                Command::Quit => {
                    cont = false;
                    Reply::Ok
                }
            })
            .await
        {
            cont = false;
        }

        if let Some((key, url, path)) = current_task {
            let key = key;
            download_task(&client, &mut rx, key, url, path)
                .await
                .unwrap();
            current_task = None;
        }
    }
}

async fn download_task<K: Clone + Send>(
    client: &Client,
    rx: &mut RpcReceiver<Command<K>, Reply<K>>,
    key: K,
    url: Url,
    path: PathBuf,
) -> Result<(), DownloadError> {
    let file_name = if let Some(file_name) = path.file_name() {
        file_name.to_string_lossy()
    } else {
        return Err(DownloadError::PathError);
    };

    let resp = client.get(url.clone()).send().await?.error_for_status()?;

    let headers = resp.headers();
    let content_type: String = if let Some(value) = headers.get(reqwest::header::CONTENT_TYPE) {
        value.to_str().unwrap().to_owned()
    } else {
        "unknown".to_owned()
    };

    let size: usize = headers
        .get(reqwest::header::CONTENT_LENGTH)
        .unwrap()
        .to_str()
        .unwrap()
        .parse()
        .unwrap();

    let root = path.parent().unwrap();
    let download_fn = format!(".{}.download", file_name);
    let download_path = root.join(download_fn);
    log::debug!("Downloading {} to {}", url, download_path.to_string_lossy());

    let mut hasher = Hasher::new();
    fs::create_dir_all(root)?;
    let mut output = fs::File::create(&download_path)?;

    let mut stream = resp.bytes_stream();

    let mut downloaded = 0;
    let mut chunk: usize = 0;

    while let Some(item) = stream.next().await {
        let item = item?;
        hasher.write_all(&item).unwrap();
        output.write_all(&item)?;
        downloaded += item.len();
        chunk += item.len();

        // Write digest file every chunk s.t. it can be picked up by the reader just by looking at
        // this file
        if chunk > CHUNK_SIZE {
            // TODO: Partial digest type
            let hash = hasher.finalize();
            let digest = Digest::new(path.clone(), &content_type, hash);
            // Ignore digest write errors
            let _ = digest.write(root);
        }

        rx.try_reply_once(|q| match q {
            Command::Start(_, _, _) => Reply::Error,
            Command::Stop => todo!(),
            Command::Status => Reply::Downloading {
                key: key.clone(),
                downloaded,
                size,
            },
            Command::Quit => todo!(),
        })?;
    }

    let hash = hasher.finalize();

    let digest = Digest::new(path.clone(), &content_type, hash);
    digest.write(root)?;
    fs::rename(download_path, path)?;

    Ok(())
}

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("IO error")]
    IoError(#[from] std::io::Error),

    #[error("HTTP error")]
    RequestError(#[from] reqwest::Error),

    #[error("RPC error")]
    RpcError(#[from] RpcError),

    #[error("Digest error")]
    DigestError(#[from] DigestError),

    #[error("Path error")]
    PathError,
}
