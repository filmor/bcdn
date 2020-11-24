mod digest;
pub use digest::{Digest, DigestError};

use crate::config::Config;
use globset::GlobSet;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::sync::RwLock;
use url::Url;

pub struct Cache {
    pub name: String,
    base: Url,
    patterns: GlobSet,
    path: PathBuf,
    digests: RwLock<HashMap<String, Digest>>,
}

impl Cache {
    pub fn new(name: &str, config: &Config) -> Result<Self, CacheError> {
        log::info!("Initializing cache '{}'", name);
        let entry = config.entries.get(name).ok_or(CacheError::ConfigMissing)?;
        let path = Path::new(&config.cache.root_path).join(name);
        fs::create_dir_all(&path)?;

        let patterns = entry.get_globset()?;
        let digests = RwLock::new(preprocess_existing(&path, &patterns));

        Ok(Cache {
            name: name.to_owned(),
            base: Url::parse(&entry.base_url).unwrap(),
            path,
            patterns,
            digests,
        })
    }

    pub async fn get(&self, filename: &str) -> CacheResult {
        if !self.patterns.is_match(filename) {
            return CacheResult::NotFound;
        }

        if let Some(digest) = self.digests.read().await.get(filename) {
            CacheResult::Ok(digest.clone())
        } else {
            CacheResult::NotCached
        }
    }

    pub fn get_redirect(&self, filename: &str) -> Url {
        self.base.join(filename).unwrap()
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }
}

#[derive(Error, Debug)]
pub enum CacheError {
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
