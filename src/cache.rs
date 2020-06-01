use crate::download::{download, DownloadError};
use crate::manifest::Manifest;
use reqwest::Client;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
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
    fn get<'a>(&'a self, name: &str) -> CacheResult<'a> {
        if let Some(in_work) = &self.in_work {
            if in_work == name {
                return CacheResult::InWork;
            }
        }
        if let Some(ref manifest) = self.items.get(name) {
            CacheResult::Ok(manifest)
        } else {
            CacheResult::NotCached
        }
    }

    async fn cache<'a>(&'a mut self, name: &str) -> CacheResult<'a> {
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

#[derive(Debug)]
enum CacheResult<'a> {
    Ok(&'a Manifest),
    DownloadError(DownloadError),
    InWork,
    NotCached,
}
