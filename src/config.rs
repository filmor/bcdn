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
    #[serde(default = "default_address")]
    pub address: String,
    #[serde(default = "default_cache_port")]
    pub port: u16,
    pub root_path: String,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct ProxyConfig {
    #[serde(default = "default_address")]
    pub address: String,
    #[serde(default = "default_proxy_port")]
    pub port: u16,
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

fn default_address() -> String {
    "127.0.0.1".to_owned()
}

fn default_cache_port() -> u16 {
    1337
}
fn default_proxy_port() -> u16 {
    1336
}

fn default_patterns() -> Vec<String> {
    vec!["*".to_owned()]
}
