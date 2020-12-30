
use futures_util::{StreamExt, future::join_all};
use reqwest::Client;
use std::{sync::Arc, time::Duration};
use tokio::{sync::RwLock, task::JoinHandle};

use crate::{Config, cache_server::cache::Cache, util::rpc::{RpcHandle, RpcReceiver}};
use crate::util::rpc::rpc;

use super::downloader::Downloader;

type JobKey = (String, String);
type ArcRw<T> = Arc<RwLock<T>>;

pub struct DownloadPool {
    update_task: JoinHandle<()>,
    client: Client,
    rpc: RpcHandle<Command, Reply>
}


// unsafe impl Send for DownloadPool {}
unsafe impl Sync for DownloadPool {}

impl DownloadPool {
    pub fn new(config: &Config) -> Self {
        let client = Client::new();

        if config.cache.max_downloads < 1 {
            panic!("Invalid configuration, max_downloads must be > 1");
        }
        
        let downloaders: Vec<_> = (1..=config.cache.max_downloads).map(|_| Downloader::new(client.clone()))
            .collect();
        
        let (tx, rx) = rpc();


        let update_task = tokio::task::spawn(async move {
            update_task(rx, downloaders).await;
        });

        DownloadPool {
            update_task,
            rpc: tx,
            client
        }
    }
    
    pub async fn enqueue(&self, cache: &Cache, filename: &str) -> DownloadState {
        unimplemented!();
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

async fn update_task(rx: RpcReceiver<Command, Reply>, downloaders: Vec<Downloader<(String, String)>>) {
    loop {
        // Loop over tasks and ask for status
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let states = join_all(downloaders.iter().map(|h| {
            h.status()
        })).await;

    }
}

enum Command {
    Enqueue,
    Status
}

enum Reply {
    Done
}

#[derive(Debug, Clone, Copy)]
pub struct DownloadState;

impl DownloadState {
    pub fn percentage(&self) -> i32 {
        unimplemented!()
    }
}