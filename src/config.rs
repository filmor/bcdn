use serde_derive::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub bind: String,
    pub root_path: String,
    pub entries: HashMap<String, Entry>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Entry {
    pub base_url: String,
}
