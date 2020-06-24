use crate::config::Config;
use crate::download::{download, DownloadError};
use crate::manifest::Manifest;
use reqwest::Client;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::sync::{RwLock, Semaphore};
use url::Url;

pub struct Cache {
    client: Client,
    pub name: String,
    base: Url,
    path: PathBuf,
    items: RwLock<HashMap<String, Manifest>>,

    work_sem: Semaphore,

    // TODO Make use of in_work as a Semaphore to limit the number of parallel downloads
    in_work: RwLock<Vec<String>>,
}

impl Cache {
    pub fn new(name: &str, config: &Config) -> Self {
        let entry = &config.entries[name];
        let path = Path::new(&config.cache.root_path).join(name);
        fs::create_dir_all(&path).unwrap();

        let max_parallel_downloads = 2;

        Cache {
            client: Client::new(),
            name: name.to_owned(),
            base: Url::parse(&entry.base_url).unwrap(),
            path,
            items: RwLock::new(HashMap::new()),
            work_sem: Semaphore::new(max_parallel_downloads),
            in_work: RwLock::new(Vec::new()),
        }
    }

    pub async fn get(&self, name: &str) -> CacheResult {
        if let Some(manifest) = self.items.read().await.get(name).cloned() {
            return CacheResult::Ok(manifest);
        }

        // TODO let redirect = self.base.join(name).unwrap();

        self.cache(name).await
    }

    pub async fn cache(&self, name: &str) -> CacheResult {
        let url = self.base.join(name).unwrap();
        let path = self.path.join(name);

        match download(&self.client, url, &path).await {
            Ok(manifest) => {
                let digest_path = self.path.join(format!(".{}.digest", name));
                let file = fs::File::create(digest_path).unwrap();
                serde_json::to_writer_pretty(file, &manifest).unwrap();

                let mut items = self.items.write().await;
                items.insert(name.to_owned(), manifest.clone());

                CacheResult::Ok(manifest)
            }
            Err(err) => {
                log::error!("Download error: {:?}", err);
                CacheResult::DownloadError(err)
            }
        }
    }
}

#[derive(Error, Debug)]
enum CacheError {}

#[derive(Debug)]
pub enum CacheResult {
    Ok(Manifest),
    DownloadError(DownloadError),
    NotCached { redirect: Url, in_work: bool },
}
