use std::collections::HashMap;
use std::path::PathBuf;
use url::Url;

use crate::manifest::Manifest;

pub struct Cache {
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
}

enum CacheResult<'a> {
    Ok(&'a Manifest),
    InWork,
    NotCached,
}
