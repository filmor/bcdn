mod digest;
pub use digest::{Digest, DigestError};

use crate::config::Config;
use globset::GlobSet;
use reqwest::Client;
use sled::{Db, Tree};
use std::fs;
use std::path::{Path, PathBuf};
use std::{collections::HashMap, ops::Deref};
use thiserror::Error;
use tokio::sync::RwLock;
use url::Url;
use zerocopy::{AsBytes, FromBytes, LayoutVerified, Unaligned};

#[derive(Clone)]
pub struct Cache {
    pub name: String,
    base: Url,
    patterns: GlobSet,
    path: PathBuf,
    db: Db,
}

#[derive(Clone, FromBytes, AsBytes)]
#[repr(C)]
struct CacheEntry {
    downloaded: u64,
    size: u64,
    hash: [u8; 32],
}

impl CacheEntry {
    pub fn is_done(&self) -> bool {
        self.size == self.downloaded
    }
}

impl Cache {
    pub fn new(name: &str, config: &Config) -> Result<Self, CacheError> {
        log::info!("Initializing cache '{}'", name);
        let entry = config.entries.get(name).ok_or(CacheError::ConfigMissing)?;
        let path = Path::new(&config.cache.root_path).join(name);
        fs::create_dir_all(&path)?;

        let db = sled::Config::new().path(path.join(".db")).open()?;

        let patterns = entry.get_globset()?;

        Ok(Cache {
            name: name.to_owned(),
            base: Url::parse(&entry.base_url).unwrap(),
            path,
            patterns,
            db,
        })
    }

    fn get_entry(&self, filename: &str) -> Option<CacheEntry> {
        self.db.get(filename).unwrap().map(|data| {
            let layout: LayoutVerified<_, CacheEntry> = LayoutVerified::new(&*data).unwrap();
            layout.into_ref().clone()
        })
    }

    fn set_entry(&self, filename: &str, entry: CacheEntry) {
        let entry = entry.as_bytes();
        self.db.insert(filename, entry).unwrap();
    }

    pub async fn get(&self, filename: &str) -> CacheResult {
        if !self.patterns.is_match(filename) {
            return CacheResult::NotFound;
        }

        CacheResult::NotCached
    }
    
    pub fn get_redirect(&self, filename: &str) -> Url {
        self.base.join(filename).unwrap()
    }

    // pub async fn cache(&self, name: &str) -> CacheResult {
    //     let url = self.base.join(name).unwrap();
    //     let path = self.path.join(name);

    //     match Downloader::new(&self.client, url, &path).download().await {
    //         Ok(digest) => {
    //             digest.write(&self.path).unwrap();

    //             let mut items = self.items.write().await;
    //             items.insert(name.to_owned(), digest.clone());

    //             CacheResult::Ok(digest)
    //         }
    //         Err(err) => {
    //             log::error!("Download error: {:?}", err);
    //             CacheResult::DownloadError(err)
    //         }
    //     }
    // }
}

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Sled error: {0:?}")]
    Sled(#[from] sled::Error),

    #[error("Globset error: {0:?}")]
    GlobSet(#[from] globset::Error),

    #[error("IO error: {0:?}")]
    IO(#[from] std::io::Error),

    #[error("Config entry missing")]
    ConfigMissing,
}

#[derive(Debug)]
pub enum CacheResult {
    Ok(Digest),
    Incomplete(Digest),
    NotCached,
    NotFound,
}

fn preprocess_existing<P: AsRef<Path>>(root: P, glob: &GlobSet) -> HashMap<String, Digest> {
    let root = root.as_ref();
    let mut res = HashMap::new();

    for entry in fs::read_dir(&root).unwrap() {
        let path = entry.unwrap().path();

        if glob.is_match(&path) {
            match load_from_path(&path) {
                Ok(digest) => {
                    res.insert(digest.file_name.clone(), digest);
                }
                Err(err) => log::warn!(
                    "Failed to load digest from {}: {}",
                    path.to_string_lossy(),
                    err
                ),
            };
        }
    }

    res
}

fn load_from_path(path: &Path) -> Result<Digest, DigestError> {
    let digest = Digest::for_path(&path)?;
    log::info!("Found existing file at {}", path.to_string_lossy());
    digest.verify()?;
    Ok(digest)
}
