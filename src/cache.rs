use crate::config::Config;
use crate::download::{download, DownloadError};
use crate::manifest::Manifest;
use reqwest::Client;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::sync::RwLock;
use url::Url;

pub struct Cache {
    client: Client,
    name: String,
    base: Url,
    path: PathBuf,
    items: RwLock<HashMap<String, Manifest>>,
    in_work: RwLock<Option<String>>,
}

impl Cache {
    pub fn new(name: &str, config: &Config) -> Self {
        let entry = &config.entries[name];
        Cache {
            client: Client::new(),
            name: name.to_owned(),
            base: Url::parse(&entry.base_url).unwrap(),
            path: Path::new(&config.root_path).join(name).to_owned(),
            items: RwLock::new(HashMap::new()),
            in_work: RwLock::new(None),
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
