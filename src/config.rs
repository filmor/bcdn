use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub proxy: ProxyConfig,
    pub entries: HashMap<String, Entry>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct CacheConfig {
    #[serde(default = "default_cache_bind")]
    pub bind: String,
    pub root_path: String,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct ProxyConfig {
    #[serde(default = "default_proxy_bind")]
    pub bind: String,
    pub nodes: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Entry {
    pub base_url: String,
    #[serde(default = "default_patterns")]
    pub patterns: Vec<String>,
}

use globset::{Error, Glob, GlobSet, GlobSetBuilder};
impl Entry {
    pub fn get_globset(&self) -> Result<GlobSet, Error> {
        let mut builder = GlobSetBuilder::new();

        for pattern in self.patterns.iter() {
            builder.add(Glob::new(&pattern)?);
        }

        builder.build()
    }
}

fn default_proxy_bind() -> String {
    "127.0.0.1:1336".to_owned()
}

fn default_cache_bind() -> String {
    "127.0.0.1:1337".to_owned()
}

fn default_patterns() -> Vec<String> {
    vec!["*".to_owned()]
}
