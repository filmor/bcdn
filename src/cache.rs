use crate::config::Config;
use crate::download::{download, DownloadError};
use crate::manifest::Manifest;
use reqwest::Client;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use url::Url;

pub struct Cache {
    client: Client,
    name: String,
    base: Url,
    path: PathBuf,
    items: HashMap<String, Manifest>,
    in_work: Option<String>,
}

impl Cache {
    pub fn new(name: &str, config: &Config) -> Self {
        let entry = &config.entries[name];
        Cache {
            client: Client::new(),
            name: name.to_owned(),
            base: Url::parse(&entry.base_url).unwrap(),
            path: Path::new(&config.root_path).join(name).to_owned(),
            items: HashMap::new(),
            in_work: None,
        }
    }

    pub fn get<'a>(&'a self, name: &str) -> CacheResult<'a> {
        let redirect = self.base.join(name).unwrap();

        if let Some(in_work) = &self.in_work {
            if in_work == name {
                return CacheResult::NotCached {
                    redirect,
                    in_work: true,
                };
            }
        }
        if let Some(ref manifest) = self.items.get(name) {
            CacheResult::Ok(manifest)
        } else {
            CacheResult::NotCached {
                redirect,
                in_work: false,
            }
        }
    }

    pub async fn cache<'a>(&'a mut self, name: &str) -> CacheResult<'a> {
        let url = self.base.join(name).unwrap();
        let path = self.path.join(name);

        match download(&self.client, url, &path).await {
            Ok(manifest) => {
                let digest_path = self.path.join(format!(".{}.digest", name));
                let file = fs::File::create(digest_path).unwrap();
                serde_json::to_writer_pretty(file, &manifest).unwrap();

                self.items.insert(name.to_owned(), manifest);

                CacheResult::Ok(&self.items[name])
            }
            Err(err) => CacheResult::DownloadError(err),
        }
    }
}

#[derive(Error, Debug)]
enum CacheError {}

#[derive(Debug)]
pub enum CacheResult<'a> {
    Ok(&'a Manifest),
    DownloadError(DownloadError),
    NotCached { redirect: Url, in_work: bool },
}
