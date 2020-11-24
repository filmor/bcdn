use blake3::Hasher;
use futures_util::StreamExt;
use reqwest::{Client, Url};
use std::{collections::HashMap, fs, io::Write, path::PathBuf, sync::Arc, time::Duration};
use tokio::{sync::RwLock, task, task::JoinHandle};

use crate::{cache_server::cache::Cache, Config};
use crate::{
    cache_server::cache::{Digest, DigestError},
    util::rpc::{rpc, RpcError, RpcHandle, RpcReceiver},
};

type JobKey = (String, String);
type ArcRw<T> = Arc<RwLock<T>>;
type JobsType = ArcRw<HashMap<JobKey, StatePtr>>;
type StatePtr = ArcRw<DownloadState>;

pub struct DownloadPool {
    handles: Vec<Handle>,
    update_task: JoinHandle<()>,
    jobs: JobsType,
    client: Client,
    queue: Sender<JobKey>,
}

struct Handle {
    join_handle: task::JoinHandle<()>,
    rpc: RpcHandle<Command, Reply>,
}

enum Command {
    Start((String, String), Url, PathBuf, StatePtr),
    Stop,
    Status,
    Quit,
}

enum Reply {
    Ok,
    Error,
    Downloading((String, String)),
    Idle,
}

// unsafe impl Send for DownloadPool {}
unsafe impl Sync for DownloadPool {}

impl DownloadPool {
    pub fn new(config: &Config) -> Self {
        let client = Client::new();

        if config.cache.max_downloads < 1 {
            panic!("Invalid configuration, max_downloads must be > 1");
        }

        let handles = (1..=config.cache.max_downloads)
            .map(|_i| {
                let client = client.clone();
                let (tx, rx) = rpc();
                let join_handle = tokio::task::spawn(async move {
                    idle_task(client, rx).await;
                });
                Handle {
                    rpc: tx,
                    join_handle,
                }
            })
            .collect();

        let jobs: JobsType = Default::default();

        let t_jobs = jobs.clone();
        let update_task = tokio::task::spawn(async move {
            update_task(t_jobs).await;
        });

        DownloadPool {
            handles,
            jobs,
            update_task,
            client,
        }
    }

    pub async fn enqueue(&self, cache: &Cache, filename: &str) -> DownloadState {
        let key = (cache.name.clone(), filename.to_owned());
        log::info!("Enqueuing key {:?}", key);
        let exists = {
            self.jobs.read().await.contains_key(&key)
        };

        if exists {
            let state = self.jobs.read().await[&key];
            log::info!("Already in work");
            state.read().await.clone()
        } else {
            log::info!("Not enqueued yet, creating new state");
            let state = Arc::new(RwLock::new(DownloadState::NotStarted));
            self.jobs.write().await.insert(key, state.clone());
            log::info!("Successfully written");
            state.clone().read().await.clone()
        }
    }
}

#[derive(Clone, Debug)]
pub enum DownloadState {
    NotStarted,
    InWork { downloaded: usize, size: usize },
    Finished { digest: Digest },
}

impl DownloadState {
    pub fn is_done(&self) -> bool {
        match self {
            DownloadState::Finished { digest: _ } => true,
            _ => false,
        }
    }

    pub fn percentage(&self) -> u8 {
        match self {
            DownloadState::NotStarted => 0,
            DownloadState::InWork { downloaded, size } => {
                ((*downloaded as f32) / (*size as f32)) as u8
            }
            DownloadState::Finished { digest: _ } => 100,
        }
    }

    fn add_downloaded(&mut self, to_add: usize) {
        match self {
            DownloadState::InWork {
                ref mut downloaded,
                size: _,
            } => *downloaded += to_add,
            _ => panic!("Download state was {:?}, must be in work", self),
        }
    }
}

async fn update_task(jobs: JobsType) {
    loop {
        tokio::time::delay_for(Duration::from_millis(100)).await;
    }
}

async fn idle_task(client: Client, mut rx: RpcReceiver<Command, Reply>) {
    let mut cont = true;
    let mut current_task = None;
    while cont {
        if let Err(_) = rx
            .reply(|q| match q {
                Command::Start(key, url, path, status) => {
                    current_task = Some((key, url, path, status));
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

        if let Some((key, url, path, status)) = current_task {
            download_task(&client, &mut rx, key, url, path, status)
                .await
                .unwrap();
            current_task = None;
        }
    }
}

async fn download_task(
    client: &Client,
    rx: &mut RpcReceiver<Command, Reply>,
    key: (String, String),
    url: Url,
    path: PathBuf,
    status: StatePtr,
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

    let size = headers
        .get(reqwest::header::CONTENT_LENGTH)
        .unwrap()
        .to_str()
        .unwrap()
        .parse()
        .unwrap();

    *status.write().await = DownloadState::InWork {
        downloaded: 0,
        size,
    };

    let root = path.parent().unwrap();
    let download_fn = format!(".{}.download", file_name);
    let download_path = root.join(download_fn);
    log::debug!("Downloading {} to {}", url, download_path.to_string_lossy());

    let mut hasher = Hasher::new();
    fs::create_dir_all(root)?;
    let mut output = fs::File::create(&download_path)?;

    let mut stream = resp.bytes_stream();

    while let Some(item) = stream.next().await {
        let item = item?;
        hasher.write_all(&item).unwrap();
        output.write_all(&item)?;
        status.write().await.add_downloaded(item.len());

        rx.try_reply(|q| match q {
            Command::Start(_, _, _, _) => Reply::Error,
            Command::Stop => todo!(),
            Command::Status => Reply::Downloading(key.clone()),
            Command::Quit => todo!(),
        })?;
    }

    let hash = hasher.finalize();

    let digest = Digest::new(path.clone(), &content_type, hash);
    digest.write(root)?;
    fs::rename(download_path, path)?;

    *status.write().await = DownloadState::Finished { digest };

    Ok(())
}

use thiserror::Error;

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
