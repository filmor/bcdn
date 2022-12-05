use futures_util::{future::join_all, StreamExt};
use reqwest::{Client, Url};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{sync::RwLock, task::JoinHandle};

use crate::util::rpc::rpc;
use crate::{
    cache_server::cache::Cache,
    util::rpc::{RpcHandle, RpcReceiver},
    Config,
};

use super::{downloader::Downloader, job_queue::JobQueue};

type JobKey = (String, String);
type ArcRw<T> = Arc<RwLock<T>>;

pub struct DownloadPool {
    update_task: JoinHandle<()>,
    client: Client,
    rpc: RpcHandle<Command, Reply>,
    base_urls: HashMap<String, Url>,
}

// unsafe impl Send for DownloadPool {}
unsafe impl Sync for DownloadPool {}

impl DownloadPool {
    pub fn new(config: &Config) -> Self {
        let client = Client::new();

        if config.cache.max_downloads < 1 {
            panic!("Invalid configuration, max_downloads must be > 1");
        }

        let downloaders: Vec<_> = (1..=config.cache.max_downloads)
            .map(|_| Downloader::new(client.clone()))
            .collect();

        let base_urls = config
            .entries
            .iter()
            .map(|(name, entry)| {
                (
                    name.clone(),
                    Url::parse(&entry.base_url).expect("Could not parse url"),
                )
            })
            .collect();

        let (tx, rx) = rpc();

        let update_task = tokio::task::spawn(async move {
            update_task(rx, downloaders).await;
        });

        DownloadPool {
            update_task,
            rpc: tx,
            client,
            base_urls,
        }
    }

    pub async fn enqueue(&self, cache: &Cache, filename: &str) -> DownloadState {
        let base = self.base_urls.get(&cache.name).expect("Invalid cache name");
        let url = base.join(filename).unwrap();
        let key = (cache.name.clone(), filename.to_owned());
        let _ = self.rpc.call(Command::Enqueue { key, url }).await.unwrap();

        DownloadState {}
    }

    // pub async fn enqueue(&self, cache: &Cache, filename: &str) -> DownloadState {
    //     let key = (cache.name.clone(), filename.to_owned());
    //     log::info!("Enqueuing key {:?}", key);
    //     let exists = {
    //         self.jobs.read().await.contains_key(&key)
    //     };

    //     if exists {
    //         let state = self.jobs.read().await[&key].clone();
    //         log::info!("Already in work");
    //         let state_val = state.read().await.clone();
    //         state_val
    //     } else {
    //         log::info!("Not enqueued yet, creating new state");
    //         let state = Arc::new(RwLock::new(DownloadState::NotStarted));
    //         self.jobs.write().await.insert(key, state.clone());
    //         log::info!("Successfully written");
    //         state.clone().read().await.clone()
    //     }
    // }
}

async fn update_task(
    mut rx: RpcReceiver<Command, Reply>,
    downloaders: Vec<Downloader<(String, String)>>,
) {
    let mut jq = JobQueue::new(10);
    loop {
        let mut cont = true;
        rx.try_reply_once(|command| {
            match command {
                Command::Enqueue { key, url } => {
                    jq.push(key, url);
                    // TODO: If already in work, get current status
                }
                Command::Quit => {
                    cont = false;
                }
            };
            Reply::Done
        })
        .unwrap();

        let states = join_all(downloaders.iter().map(|h| h.status())).await;

        // TODO: Handle Enqueue, Status and Quit

        // Loop over tasks and ask for status
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

enum Command {
    Enqueue { key: (String, String), url: Url },
    Quit,
}

enum Reply {
    Done,
}

#[derive(Debug, Clone, Copy)]
pub struct DownloadState;

impl DownloadState {
    pub fn percentage(&self) -> i32 {
        unimplemented!()
    }
}
