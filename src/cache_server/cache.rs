use super::download::{download, DownloadError};
use crate::config::Config;
use crate::digest::Digest;
use globset::GlobSet;
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
    patterns: GlobSet,
    path: PathBuf,
    items: RwLock<HashMap<String, Digest>>,
    // work_sem: Semaphore,

    // TODO Make use of in_work as a Semaphore to limit the number of parallel downloads
    // in_work: RwLock<Vec<String>>,
}

impl Cache {
    pub fn new(name: &str, config: &Config) -> Self {
        let entry = &config.entries[name];
        let path = Path::new(&config.cache.root_path).join(name);
        fs::create_dir_all(&path).unwrap();
        let patterns = entry.get_globset().unwrap();

        let items = preprocess_existing(&path, &patterns);

        let max_parallel_downloads = 2;

        Cache {
            client: Client::new(),
            name: name.to_owned(),
            base: Url::parse(&entry.base_url).unwrap(),
            path,
            patterns,
            items: RwLock::new(items),
            // work_sem: Semaphore::new(max_parallel_downloads),
            // in_work: RwLock::new(Vec::new()),
        }
    }

    pub async fn get(&self, filename: &str) -> CacheResult {
        if !self.patterns.is_match(filename) {
            return CacheResult::NotFound;
        }

        if let Some(digest) = self.items.read().await.get(filename).cloned() {
            return CacheResult::Ok(digest);
        }

        self.cache(filename).await
    }

    pub async fn cache(&self, name: &str) -> CacheResult {
        let url = self.base.join(name).unwrap();
        let path = self.path.join(name);

        match download(&self.client, url, &path).await {
            Ok(digest) => {
                digest.write(&self.path).unwrap();

                let mut items = self.items.write().await;
                items.insert(name.to_owned(), digest.clone());

                CacheResult::Ok(digest)
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
    Ok(Digest),
    DownloadError(DownloadError),
    NotCached { redirect: Url, in_work: bool },
    NotFound,
}

fn preprocess_existing<P: AsRef<Path>>(root: P, glob: &GlobSet) -> HashMap<String, Digest> {
    let root = root.as_ref();
    let mut res = HashMap::new();

    for entry in fs::read_dir(&root).unwrap() {
        let path = entry.unwrap().path();

        if glob.is_match(&path) {
            let digest = Digest::for_path(&path).unwrap();
            log::info!("Found existing file at {}", path.to_string_lossy());
            digest.verify().unwrap();

            let file_name = digest.file_name.clone();
            res.insert(file_name, digest);
        }
    }

    res
}
